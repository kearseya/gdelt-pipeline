# How To

## Build

```
cd gdelt_pipe
cargo build --release
target/release/analyse --help
```

## Download

Editing START_DATE, END_DATE, and OUTPUT_DIR in `scripts/download.py`. These do not need to be unzipped to be read. For lower RAM machine use smaller chunks and batch by months (or use an include list when running node). With 49 domains `assets/domains.txt` and filtering with `assets/stop_words.txt` there are >237 million total words counted between 2013 and 2026.

## (Optional) Collecting filter words

`analyse rank -i input -n 500` to get common words to put in `assets/stop_words.txt` (default list of excluded words).

## Create node file

`analyse node -i inputDir -s outputGraphFile`

## (Optional) Merging words

```
analyse merge -g inputGraphFile --suggest -s mergeWords.txt
analyse merge -g inputGraphFile -m mergeWords.txt -s mergedGraphFile
```

This process does use stemming so check words of interest (e.g. inflation = inflated, inflatable where inflatable probably is not wanted). This file can also be made manually if there are few words of interest.

## Charting

`analyse chart -g graph1 graph2` and search words in the search bar. Use `|` (or), `&` (and), or `+ (word1 | word2)` to find combined frequency of groups of words. (`-g graphDir/*.bin` will work but may not be in the correct order with some naming conventions).

## (Optional) Plotting

`analyse plot -i inputGraphFile` to plot an individual graph. Nodes will be displayed from most to least mentioned in the centre spiraling outwards. Clicking a node (word) will create a circle of all connected words (above filter limits) around it.

This can be combined with comparing (`analyse compare --graph1 graph1File --graph2 graph2File -s compare_graph.bin`) where `compare_grpah.bin` can be plotted with the `-d/--diff` flag.
