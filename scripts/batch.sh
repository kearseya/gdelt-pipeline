dir="../gdelt"

for i in {2026..2026}; do
  ls "${dir}/gdelt_${i}"
  ${dir}/gdelt_pipe/target/release/analyse rank --input "${dir}/gdelt_${i}" --exclude ${dir}/gdelt_pipe/assets/stop_words.txt -n 1000 >${dir}/graphing/ranking/${i}_top_1000.txt
  ${dir}/gdelt_pipe/target/release/analyse node --input ${dir}/gdelt_${i} --include-domains --exclude ${dir}/gdelt_pipe/assets/stop_words.txt --domains ${dir}/gdelt_pipe/assets/domains.txt --save ${dir}/graphing/pre_merge/${i}_pre_merge.bin --low-mem
  ${dir}/gdelt_pipe/target/release/analyse merge --suggest --graph ${dir}/graphing/pre_merge/${i}_pre_merge.bin --save ${dir}/graphing/suggestions/${i}_suggest_merge.txt
  ${dir}/gdelt_pipe/target/release/analyse merge --merge-file ${dir}/graphing/suggestions/${i}_suggest_merge.txt --graph ${dir}/graphing/pre_merge/${i}_pre_merge.bin --save ${dir}/graphing/merged/${i}_merged.bin
done

## Compare
# for i in {2025..2025}; do
#   ${dir}/gdelt_pipe/target/release/analyse compare --graph1 ${dir}/graphing/merged/$((i + 1))_merged.bin --graph2 ${dir}/graphing/merged/${i}_merged.bin --save ${dir}/graphing/compares/$((i + 1))_${i}.bin
# done
