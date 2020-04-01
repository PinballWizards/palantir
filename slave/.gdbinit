set print asm-demangle on
target extended-remote :2331
monitor semihosting enable
monitor halt
load
monitor reset