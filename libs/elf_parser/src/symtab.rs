extern crate posix;
use posix::Errno;

extern crate stpack;
use stpack::{unpacker, Unpacker};

use crate::ElfSection;
use crate::err::ElfParserError;
use crate::ident::{ElfClass, ElfEndian};
use crate::string_table;
use crate::symbol::Symbol;

const SHT_SYMTAB: u32 = 2;
const SHT_STRTAB: u32 = 3;

unpacker! {
    pub struct Elf32SymtabEntry {
        pub name: u32,
        pub value: u32,
        pub size: u32,
        pub info: u8,
        pub other: u8,
        pub shndx: u16,
    }
}

unpacker! {
    pub struct Elf64SymtabEntry {
        pub name: u32,
        pub info: u8,
        pub other: u8,
        pub shndx: u16,
        pub value: u64,
        pub size: u64,
    }
}

pub struct SymtabIterator<'a> {
    class: ElfClass,
    le: bool,
    sections: &'a Vec<ElfSection<'a>>,
    curr_secidx: usize,
    curr_symidx: usize,
}

impl<'a> SymtabIterator<'a> {
    pub(crate) fn new(class: ElfClass,
                      endian: ElfEndian,
                      sections: &'a Vec<ElfSection<'a>>) -> Self
    {
        Self {
            class,
            le: endian == ElfEndian::ElfLE,
            sections,
            curr_secidx: 0,
            curr_symidx: 0,
        }
    }
}

impl<'a> Iterator for SymtabIterator<'a> {
    type Item = Result<Symbol<'a>, ElfParserError>;

    fn next(&mut self) -> Option<Self::Item> {
        let mut secidx = self.curr_secidx;
        let mut symidx = self.curr_symidx;
        let seccnt = self.sections.len();

        loop {
            if secidx >= seccnt {
                break;
            }

            let sec = &self.sections[secidx];
            if sec.typ == SHT_SYMTAB {
                if sec.entsize == 0 {
                    return Some(Err(ElfParserError::new(
                        Errno::EINVAL, String::from("Symtab section entry size is 0 (file broken)"))))
                }

                if symidx < (sec.content.len() / (sec.entsize as usize)) {
                    break;
                }
            }

            secidx += 1;
            symidx = 0;
        }

        self.curr_secidx = secidx;
        self.curr_symidx = symidx;

        if secidx >= seccnt {
            return None;
        }

        let sec = &self.sections[secidx];
        let data = &sec.content[(sec.entsize as usize * symidx)..];

        let (nameoff, value, size, info, other, shndx) = match self.class {
            ElfClass::Elf32 =>
                match Elf32SymtabEntry::unpack(data, self.le) {
                    Ok((ent, _)) => (
                        ent.name as usize,
                        ent.value as u64,
                        ent.size as u64,
                        ent.info,
                        ent.other,
                        ent.shndx,
                    ),
                    Err(_) => return Some(Err(ElfParserError::new(
                        Errno::EINVAL, String::from("Failed to parse symtab entry")))),
                },
            ElfClass::Elf64 =>
                match Elf64SymtabEntry::unpack(data, self.le) {
                    Ok((ent, _)) => (
                        ent.name as usize,
                        ent.value,
                        ent.size,
                        ent.info,
                        ent.other,
                        ent.shndx,
                    ),
                    Err(_) => return Some(Err(ElfParserError::new(
                        Errno::EINVAL, String::from("Failed to parse symtab entry")))),
                },
        };

        if sec.link as usize >= self.sections.len() {
            return Some(Err(ElfParserError::new(
                Errno::EINVAL,
                format!("Symtab refer invalid strtab section index: \
                         {} (must be less than {})",
                        sec.link, self.sections.len()))));
        }

        let strtab_sec = &self.sections[sec.link as usize];
        if strtab_sec.typ != SHT_STRTAB {
            return Some(Err(ElfParserError::new(
                Errno::EINVAL,
                format!("Symtab linked section is not SHT_STRTAB: {}", sec.link))));
        }

        let name = string_table::read_str_from_offset(
            strtab_sec.content, nameoff);

        self.curr_symidx = symidx + 1;

        Some(Ok(Symbol {
            name,
            value,
            size,
            info,
            other,
            shndx,
        }))
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        ElfSection,
        ident::{
            ElfClass,
            ElfEndian,
        },
        symtab::{
            SymtabIterator,
            Elf32SymtabEntry,
            Elf64SymtabEntry,
            SHT_SYMTAB,
            SHT_STRTAB,
        },
        symbol::Symbol,
        stpack::Unpacker,
    };

    #[test]
    fn elf32be_first_section_is_zero_length_symtab() {
        let sections = vec![

            ElfSection {
                name: "",
                typ: SHT_SYMTAB,
                flags: 0,
                addr: 0,
                link: 2,
                info: 0,
                addralign: 0,
                entsize: Elf32SymtabEntry::SIZE as u64,
                content: &[] as &[u8],
            },

            ElfSection {
                name: "",
                typ: SHT_SYMTAB,
                flags: 0,
                addr: 0,
                link: 2,
                info: 0,
                addralign: 0,
                entsize: Elf32SymtabEntry::SIZE as u64,
                content: &[
                    0, 0, 0, 1,                 // name
                    0x11, 0x22, 0x33, 0x44,     // addr
                    0, 0, 0, 0,                 // size
                    0,                          // info
                    0,                          // other
                    0, 0,                       // shndx
                ],
            },

            ElfSection {
                name: "",
                typ: SHT_STRTAB,
                flags: 0,
                addr: 0,
                link: 0,
                info: 0,
                addralign: 0,
                entsize: 0,
                content: &[
                    0,
                    b't', b'e', b's', b't', 0,
                ],
            },

        ];

        assert_eq!(
            SymtabIterator::new(ElfClass::Elf32,
                                ElfEndian::ElfBE,
                                &sections)
                .map(|r| r.unwrap())
                .collect::<Vec<Symbol>>(),
            vec![
                Symbol {
                    name: "test",
                    value: 0x11223344u64,
                    size: 0,
                    info: 0,
                    other: 0,
                    shndx: 0,
                },
            ]
        );
    }

    #[test]
    fn elf32be_invalid_symtab_entsize() {
        let sections = vec![

            ElfSection {
                name: "",
                typ: SHT_SYMTAB,
                flags: 0,
                addr: 0,
                link: 1,
                info: 0,
                addralign: 0,
                entsize: 0,
                content: &[
                    0, 0, 0, 1,                 // name
                    0x11, 0x22, 0x33, 0x44,     // addr
                    0, 0, 0, 0,                 // size
                    0,                          // info
                    0,                          // other
                    0, 0,                       // shndx
                ],
            },

            ElfSection {
                name: "",
                typ: SHT_STRTAB,
                flags: 0,
                addr: 0,
                link: 0,
                info: 0,
                addralign: 0,
                entsize: 0,
                content: &[
                    0,
                    b't', b'e', b's', b't', 0,
                ],
            },

        ];

        let mut iter =
            SymtabIterator::new(
                ElfClass::Elf32, ElfEndian::ElfBE, &sections);

        iter.next().unwrap().expect_err(
            "Parsing broken symtab unexpectedly succeed");
    }

    #[test]
    fn elf32be_incomplete_symtab() {
        let sections = vec![

            ElfSection {
                name: "",
                typ: SHT_SYMTAB,
                flags: 0,
                addr: 0,
                link: 1,
                info: 0,
                addralign: 0,
                entsize: Elf32SymtabEntry::SIZE as u64 - 1,
                content: &[
                    0, 0, 0, 1,                 // name
                    0x11, 0x22, 0x33, 0x44,     // addr
                    0, 0, 0, 0,                 // size
                    0,                          // info
                    0,                          // other
                    0, // 0,                       // shndx
                ],
            },

            ElfSection {
                name: "",
                typ: SHT_STRTAB,
                flags: 0,
                addr: 0,
                link: 0,
                info: 0,
                addralign: 0,
                entsize: 0,
                content: &[
                    0,
                    b't', b'e', b's', b't', 0,
                ],
            },

        ];

        let mut iter =
            SymtabIterator::new(
                ElfClass::Elf32, ElfEndian::ElfBE, &sections);

        iter.next().unwrap().expect_err(
            "Parsing broken symtab unexpectedly succeed");
    }

    #[test]
    fn elf32be_symtab_link_out_of_range() {
        let sections = vec![

            ElfSection {
                name: "",
                typ: SHT_SYMTAB,
                flags: 0,
                addr: 0,
                link: 2,
                info: 0,
                addralign: 0,
                entsize: Elf32SymtabEntry::SIZE as u64,
                content: &[
                    0, 0, 0, 1,                 // name
                    0x11, 0x22, 0x33, 0x44,     // addr
                    0, 0, 0, 0,                 // size
                    0,                          // info
                    0,                          // other
                    0, 0,                       // shndx
                ],
            },

            ElfSection {
                name: "",
                typ: SHT_STRTAB,
                flags: 0,
                addr: 0,
                link: 0,
                info: 0,
                addralign: 0,
                entsize: 0,
                content: &[
                    0,
                    b't', b'e', b's', b't', 0,
                ],
            },

        ];

        let mut iter =
            SymtabIterator::new(
                ElfClass::Elf32, ElfEndian::ElfBE, &sections);

        iter.next().unwrap().expect_err(
            "Parsing broken symtab unexpectedly succeed");
    }

    #[test]
    fn elf64le() {
        let sections = vec![

            ElfSection {
                name: "",
                typ: SHT_SYMTAB,
                flags: 0,
                addr: 0,
                link: 1,
                info: 0,
                addralign: 0,
                entsize: Elf64SymtabEntry::SIZE as u64,
                content: &[
                    1, 0, 0, 0,                 // name
                    0,                          // info
                    0,                          // other
                    0, 0,                       // shndx
                    0xff, 0xee, 0xdd, 0xcc,     // addr
                    0xbb, 0xaa, 0x99, 0x88,     // addr
                    0, 0, 0, 0,                 // size
                    0, 0, 0, 0,                 // size
                ],
            },

            ElfSection {
                name: "",
                typ: SHT_STRTAB,
                flags: 0,
                addr: 0,
                link: 0,
                info: 0,
                addralign: 0,
                entsize: 0,
                content: &[
                    0,
                    b't', b'e', b's', b't', 0,
                ],
            },

        ];

        assert_eq!(
            SymtabIterator::new(ElfClass::Elf64,
                                ElfEndian::ElfLE,
                                &sections)
                .map(|r| r.unwrap())
                .collect::<Vec<Symbol>>(),
            vec![
                Symbol {
                    name: "test",
                    value: 0x8899aabb_ccddeeffu64,
                    size: 0,
                    info: 0,
                    other: 0,
                    shndx: 0,
                },
            ]
        );
    }

    #[test]
    fn elf64le_incomplete_symtab() {
        let sections = vec![

            ElfSection {
                name: "",
                typ: SHT_SYMTAB,
                flags: 0,
                addr: 0,
                link: 1,
                info: 0,
                addralign: 0,
                entsize: Elf64SymtabEntry::SIZE as u64 - 1,
                content: &[
                    1, 0, 0, 0,                 // name
                    0,                          // info
                    0,                          // other
                    0, 0,                       // shndx
                    0xff, 0xee, 0xdd, 0xcc,     // addr
                    0xbb, 0xaa, 0x99, 0x88,     // addr
                    0, 0, 0, 0,                 // size
                    0, 0, 0, // 0,                 // size
                ],
            },

            ElfSection {
                name: "",
                typ: SHT_STRTAB,
                flags: 0,
                addr: 0,
                link: 0,
                info: 0,
                addralign: 0,
                entsize: 0,
                content: &[
                    0,
                    b't', b'e', b's', b't', 0,
                ],
            },

        ];

        let mut iter =
            SymtabIterator::new(
                ElfClass::Elf64, ElfEndian::ElfLE, &sections);

        iter.next().unwrap().expect_err(
            "Parsing broken symtab unexpectedly succeed");
    }

    #[test]
    fn elf32be_shstrtab_invalid_type() {
        let sections = vec![

            ElfSection {
                name: "",
                typ: SHT_SYMTAB,
                flags: 0,
                addr: 0,
                link: 1,
                info: 0,
                addralign: 0,
                entsize: Elf32SymtabEntry::SIZE as u64,
                content: &[
                    0, 0, 0, 1,                 // name
                    0x11, 0x22, 0x33, 0x44,     // addr
                    0, 0, 0, 0,                 // size
                    0,                          // info
                    0,                          // other
                    0, 0,                       // shndx
                ],
            },

            ElfSection {
                name: "",
                typ: SHT_SYMTAB,
                flags: 0,
                addr: 0,
                link: 0,
                info: 0,
                addralign: 0,
                entsize: 0,
                content: &[
                    0,
                    b't', b'e', b's', b't', 0,
                ],
            },

        ];

        let mut iter =
            SymtabIterator::new(
                ElfClass::Elf32, ElfEndian::ElfBE, &sections);

        iter.next().unwrap().expect_err(
            "Parsing broken symtab unexpectedly succeed");
    }
}