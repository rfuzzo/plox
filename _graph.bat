@echo off

sccmap ./tmp/scc_base/graphviz.dot -o ./tmp/scc_base.dot
dot -Tpng ./tmp/scc_base.dot -O

sccmap ./tmp/scc_user/graphviz.dot -o ./tmp/scc_user.dot
dot -Tpng ./tmp/scc_user.dot -O

sccmap ./tmp/scc_full/graphviz.dot -o ./tmp/scc_full.dot
dot -Tpng ./tmp/scc_full.dot -O

echo "Graphs generated"
pause