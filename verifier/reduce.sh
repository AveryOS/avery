echo > empty.txt
bugpoint -compile-custom -mlimit=2000 -compile-command="bash reduce-cmd.sh" -output empty.txt $1 
llvm-dis bugpoint-reduced-simplified.bc
llc bugpoint-reduced-simplified.bc -filetype=obj
./target/release/verify -b -f bugpoint-reduced-simplified.o