@echo off

:: run clean
call _clean.bat

cd ./tmp

sccmap ./scc_base/graphviz.dot -o ./scc_base.dot
dot -Tpng ./scc_base.dot -O

echo "Graphs generated for base"

sccmap ./scc_user/graphviz.dot -o ./scc_user.dot
dot -Tpng ./scc_user.dot -O

echo "Graphs generated for user"

sccmap ./scc_full/graphviz.dot -o ./scc_full.dot
dot -Tpng ./scc_full.dot -O

echo "Graphs generated for full"

pause