#!/usr/bin/env bash

set -euo pipefail

user="${1:?missing listenbrainz username}"

cache_dir="${XDG_CACHE_HOME:-$HOME/.cache}/ἐννεάς-listenbrainz-watcher"
mkdir -p "$cache_dir"

load-cached-info() {
  local file="$cache_dir/$user.playing-now.json"
  [[ -f "$file" ]] && cat "$file" || true
}

save-cached-info() {
  local file="$cache_dir/$user.playing-now.json"
  echo "$info" >"$file"
}

get-info() {
  curl -s "https://api.listenbrainz.org/1/user/$user/playing-now" | jq -Mc '
    .payload.listens[].track_metadata
    | {
      artist: .artist_name,
      release: .release_name,
      mbid: .additional_info.release_mbid,
    }
  '
}

info="$(load-cached-info)"
update-info() {
  if local new="$(get-info)" && [ "$info" != "$new" ]
  then
    info="$new"
    save-cached-info
    return 0
  else
    return 1
  fi
}

query() {
  jq -rMc "$1" <<<"$info"
}

image=
change-image() {
  local artist="$(query .artist)"
  local album="$(query .release)"

  if [ -z "$artist" ] || [ -z "$album" ]
  then
    return 1
  fi

  echo "Listening to $artist - $album"

  local new="$(beet list -a -f '$artpath' "albumartists::^$artist\$" "album::^$album\$")"
  if [ -z "$new" ]
  then
    echo "Could not find album art"
    return 1
  fi

  if [ "$image" != "$new" ]
  then
    image="$new"
    echo "Displaying $image"
    cargo run -q -- "$image"
    return 0
  fi

  return 1
}

while true
do
  wait=30
  if update-info
  then
    if change-image
    then
      wait=60
    fi
  fi

  for (( i = wait; i > 0; i-- ))
  do
    printf '·'
  done

  for (( i = wait; i > 0; i-- ))
  do
    if read -st 1
    then
      printf '\r'
      for (( ; i > 0; i-- ))
      do
        printf ' '
      done
      printf '\r'
      break
    fi
    printf '\b \b'
  done
done
