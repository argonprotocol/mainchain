#!/bin/bash

TAG_NAME=$(git describe --tags --exact-match 2>/dev/null)

if [ -z "$TAG_NAME" ]; then
  echo "Usage: $0 <tag>"
  exit 1
fi

git add -A
git commit --amend --no-edit
git tag -f $TAG_NAME
git push --force origin HEAD --tags
