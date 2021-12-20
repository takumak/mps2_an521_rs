#[macro_export]
macro_rules! unpacker {
    {@constructor_from <le> $ftyp:ty} => {
        <$ftyp>::from_le_bytes
    };

    {@constructor_from <be> $ftyp:ty} => {
        <$ftyp>::from_be_bytes
    };

    {@constructor_one <$lebe:ident> $data:ident, $ftyp:ty { $($p:tt)* }} => {
        unpacker!{@constructor_from <$lebe> $ftyp}
        (<[u8; core::mem::size_of::<$ftyp>()]>::try_from(
            &$data[(unpacker!{@allsize $($p)*})..
                   (unpacker!{@allsize $($p)*}+(core::mem::size_of::<$ftyp>()))]
        ).unwrap())
    };

    {@constructor <$lebe:ident> $data:ident
     { $($result:tt)* },
     { $($p:tt)* },
     { }} =>
    {
        Self { $($result)* }
    };

    {@constructor <$lebe:ident> $data:ident
     { $($result:tt)* },
     { $($p:tt)* },
     { $fname:ident : $ftyp:ty }} =>
    {
        unpacker!{@constructor <$lebe> $data {$($result)*}, {$($p)*}, {$fname : $ftyp,}}
    };

    {@constructor <$lebe:ident> $data:ident
     { $($result:tt)* },
     { $($p:tt)* },
     { $fname:ident : $ftyp:ty, $($body:tt)* }} =>
    {
        unpacker!{
            @constructor <$lebe> $data
            {$($result)*
             $fname: unpacker!{@constructor_one <$lebe> $data, $ftyp { $($p)* }},},
            {$($p)* $fname : $ftyp, },
            { $($body)* }}
    };

    {@allsize} => {
        0
    };

    {@allsize $fname:ident : $ftyp:ty} => {
        unpacker!{@allsize $fname : $ftyp,}
    };

    {@allsize $fname:ident : $ftyp:ty, $($body:tt)*} => {
        core::mem::size_of::<$ftyp>() + unpacker!{@allsize $($body)*}
    };

    {$(#[$attr:meta])* pub struct $stname:ident { $($body:tt)* }} => {
        $(#[$attr])* pub unpacker!{struct $stname { $($body)* }}
    };

    {$(#[$attr:meta])* struct $stname:ident { $($body:tt)* }} => {
        $(#[$attr])*
        struct $stname { $($body)* }
        impl $stname {
            const SIZE: usize = unpacker!{@allsize $($body)*};

            fn unpack_le(data: &[u8]) -> Result<(Self, &[u8]), ()> {
                if data.len() < Self::SIZE {
                    Err(())
                } else {
                    let (data, right) = data.split_at(Self::SIZE);
                    Ok((
                        unpacker!{@constructor <le> data { }, { }, { $($body)* }},
                        right
                    ))
                }
            }

            fn unpack_be(data: &[u8]) -> Result<(Self, &[u8]), ()> {
                if data.len() < Self::SIZE {
                    Err(())
                } else {
                    let (data, right) = data.split_at(Self::SIZE);
                    Ok((
                        unpacker!{@constructor <be> data { }, { }, { $($body)* }},
                        right
                    ))
                }
            }
        }
    };
}

#[cfg(test)]
mod tests {
    unpacker! {
        #[derive(PartialEq, Eq, Debug)]
        struct Foo {
            foo: u8,
            bar: u16,
            baz: u32,
        }
    }

    #[test]
    fn foo_size() {
        assert_eq!(Foo::SIZE, 7);
    }

    #[test]
    fn foo_le() {
        let data: Vec<u8> = (0..10).collect();
        assert_eq!(
            Foo::unpack_le(&data),
            Ok((
                Foo {
                    foo: 0x00,
                    bar: 0x0201,
                    baz: 0x06050403,
                },
                &[7u8, 8u8, 9u8] as &[u8]
            ))
        );
    }

    #[test]
    fn test_be() {
        let data: Vec<u8> = (0..10).collect();
        assert_eq!(
            Foo::unpack_be(&data),
            Ok((
                Foo {
                    foo: 0x00,
                    bar: 0x0102,
                    baz: 0x03040506,
                },
                &[7u8, 8u8, 9u8] as &[u8]
            ))
        );
    }
}
