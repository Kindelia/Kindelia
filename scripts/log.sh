rm -rf ~/.kindelia
cargo run --release -- start --mine > log1.txt& PID1=$!; sleep $1; 
cargo run --release -- start --mine > log2.txt& PID2=$!; sleep $1;
cargo run --release -- start --mine > log3.txt& PID3=$!; sleep $1;
cargo run --release -- start --mine > log4.txt& PID4=$!; sleep $1;
sed 's/\~\~/node1/g' log1.txt > log1.txt.tmp; mv log1.txt.tmp log1.txt;
sed 's/\~\~/node2/g' log2.txt > log2.txt.tmp; mv log2.txt.tmp log2.txt;
sed 's/\~\~/node3/g' log3.txt > log3.txt.tmp; mv log3.txt.tmp log3.txt;
sed 's/\~\~/node4/g' log4.txt > log4.txt.tmp; mv log4.txt.tmp log4.txt;
cat log1.txt log2.txt log3.txt log4.txt > log.txt.tmp; cat log.txt.tmp | sort -n -b > log.txt; rm log.txt.tmp;
kill $PID1; kill $PID2; kill $PID3; kill $PID4;
# remove file if flag is passed
if [ "$2" = "--remove" ]; then
    rm log1.txt log2.txt log3.txt log4.txt
fi
