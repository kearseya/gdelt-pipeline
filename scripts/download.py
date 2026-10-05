#!/usr/bin/env python3
"""
Download GDELT daily event files for a given date range.

GDELT publishes one zip file per day at:
    http://data.gdeltproject.org/events/YYYYMMDD.export.CSV.zip

This script downloads all files in a date range into a local folder,
skipping files that already exist.

Usage:
    python download.py
"""

import os
import time
import requests
from datetime import date, timedelta

# ---- Configuration ----
START_DATE = date(2025, 1, 1)
END_DATE = date(2025, 12, 31)
OUTPUT_DIR = "../gdelt_2025"

BASE_URL = "http://data.gdeltproject.org/events/{}.export.CSV.zip"
MAX_RETRIES = 3
DELAY_SECONDS = 1.0
TIMEOUT_SECONDS = 30


def daterange(start, end):
    """Yield each date from start to end, inclusive."""
    days = (end - start).days
    for n in range(days + 1):
        yield start + timedelta(days=n)


def download_file(url, dest_path):
    """Download a single file with retries. Returns True on success."""
    for attempt in range(1, MAX_RETRIES + 1):
        try:
            response = requests.get(url, timeout=TIMEOUT_SECONDS)
            if response.status_code == 200:
                with open(dest_path, "wb") as f:
                    f.write(response.content)
                return True
            elif response.status_code == 404:
                print(f"  Not found (404): {url}")
                return False
            else:
                print(f"  Attempt {attempt}: HTTP {response.status_code} for {url}")
        except requests.exceptions.RequestException as e:
            print(f"  Attempt {attempt} failed: {e}")
        time.sleep(2 * attempt)  # backoff
    return False


def main():
    os.makedirs(OUTPUT_DIR, exist_ok=True)

    total = (END_DATE - START_DATE).days + 1
    downloaded, skipped, failed = 0, 0, 0

    for i, day in enumerate(daterange(START_DATE, END_DATE), start=1):
        date_str = day.strftime("%Y%m%d")
        filename = f"{date_str}.export.CSV.zip"
        dest_path = os.path.join(OUTPUT_DIR, filename)
        csv_filename = f"{date_str}.export.CSV"
        csv_dest_path = os.path.join(OUTPUT_DIR, csv_filename)
        url = BASE_URL.format(date_str)

        if os.path.exists(dest_path):
            skipped += 1
            continue
        if os.path.exists(csv_dest_path):
            skipped += 1
            continue

        print(f"[{i}/{total}] Downloading {filename} ...")
        success = download_file(url, dest_path)

        if success:
            downloaded += 1
        else:
            failed += 1

        time.sleep(DELAY_SECONDS)

    print("\nDone.")
    print(f"  Downloaded: {downloaded}")
    print(f"  Skipped (already existed): {skipped}")
    print(f"  Failed: {failed}")


if __name__ == "__main__":
    main()
