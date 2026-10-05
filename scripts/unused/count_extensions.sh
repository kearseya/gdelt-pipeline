awk -F',' '{
    url=$NF;
    if (match(url, /\.([A-Za-z0-9]+)(\?|$)/, ext))
        print tolower(ext[1]);
}' csv_dir/*.csv | sort | uniq -c | sort -nr
