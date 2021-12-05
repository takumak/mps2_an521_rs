MEMORY
{
    VECTOR   : ORIGIN = 0x10000000, LENGTH = 4K
    ROM      : ORIGIN = 0x38000000, LENGTH = 64K
    RAM      : ORIGIN = 0x38010000, LENGTH = 64K
    STACK    : ORIGIN = 0x38020000, LENGTH = 64K
    KALLSYMS : ORIGIN = 0x38030000, LENGTH = 64K
}

SECTIONS
{
    .vector_table ORIGIN(VECTOR) :
    {
        KEEP(*(.vector_table));
    } > VECTOR

    .rom ORIGIN(ROM) :
    {
        __text_s = .;
        *(.text .text.*);
        __text_e = .;

        /* for `-C relocation-model=pic` */
        *(.got .got.*);

        . = ALIGN(4);
        __rodata_s = .;
        *(.rodata .rodata.*);
        . = ALIGN(4);
        __rodata_e = .;
    } > ROM

    .ram ORIGIN(RAM) :
    {
        . = ALIGN(4);
        __data_s = .;
        *(.data .data.*);
        . = ALIGN(4);
        __data_e = .;

        . = ALIGN(4);
        __bss_s = .;
        *(.bss .bss.*);
        . = ALIGN(4);
        __bss_e = .;
    } > RAM

    .stack ORIGIN(STACK) :
    {
        __stack_s = .;
        . += LENGTH(STACK);
        __stack_e = .;
    } > STACK

    .kallsyms ORIGIN(KALLSYMS) :
    {
        __kallsyms = .;
        . += LENGTH(KALLSYMS);
    } > KALLSYMS

    /DISCARD/ :
    {
        *(.ARM.exidx);
        *(.ARM.exidx.*);
        *(.ARM.extab.*);
    }
}

PROVIDE(__nmi         = DefaultExceptionHandler);
PROVIDE(__hardfault   = DefaultExceptionHandler);
PROVIDE(__memmanage   = DefaultExceptionHandler);
PROVIDE(__busfault    = DefaultExceptionHandler);
PROVIDE(__usagefault  = DefaultExceptionHandler);
PROVIDE(__securefault = DefaultExceptionHandler);
PROVIDE(__svc         = DefaultExceptionHandler);
PROVIDE(__debugmon    = DefaultExceptionHandler);
PROVIDE(__pendsv      = DefaultExceptionHandler);
PROVIDE(__systick     = DefaultExceptionHandler);
