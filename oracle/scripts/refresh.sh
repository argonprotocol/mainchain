#!/bin/bash

# Define the URL and output file
URL="https://www.bls.gov/schedule/news_release/cpi.htm"
OUTPUT="$(dirname "$0")/../cpi_schedule.html"


if [ -x "$(command -v google-chrome)" ]; then
    CHROME_PATH="$(command -v google-chrome)"
elif [ -x "$(command -v chromium-browser)" ]; then
    CHROME_PATH="$(command -v chromium-browser)"
elif [ -x "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome" ]; then
    CHROME_PATH="/Applications/Google Chrome.app/Contents/MacOS/Google Chrome"
else
    echo "Chrome or Chromium browser not found. Please install it."
    exit 1
fi

"$CHROME_PATH" --headless=new --dump-dom $URL > $OUTPUT

  echo "Save the content to $OUTPUT"
