target remote :3333
set print asm-demangle on
set print pretty on
load
monitor tpiu config internal itm.txt uart off 84000000
monitor itm port 0 on
break main
continue
