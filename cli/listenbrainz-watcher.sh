#!/usr/bin/env bash

set -euo pipefail

user="${1:?missing listenbrainz username}"
source="${2:?missing album art source}"

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
  local new="$(get-info)"
  [ "$info" != "$new" ] || return 1
  info="$new"
  save-cached-info
}

query() {
  jq -rMc "$1" <<<"$info"
}

get-image-beets() {
  local artist="$(query .artist)"
  local album="$(query .release)"

  [ -n "$artist" ] || return 1
  [ -n "$album" ] || return 1

  local image="$(beet list -a -f '$artpath' "albumartists::^$artist\$" "album::^$album\$")"

  [ -n "$image" ] || return 1

  echo "$image"
}

get-image-coverartarchive() {
  local mbid="$(query .mbid)"
  local file="$cache_dir/$mbid.cover.jpg" # probably jpg, but maybe not, doesn't really matter

  [ -n "$mbid" ] || return 1

  if ! [[ -f "$file" ]]
  then
    local url="https://coverartarchive.org/release/$mbid/front"
    curl -L -o "$file" "$url" || return 1
  fi

  echo "$file"
}

log() {
  local artist="$(query .artist)"
  local album="$(query .release)"

  echo "Listening to $artist - $album"
}

image=
change-image() {
  local new="$("get-image-$source")"

  [ -n "$new" ] && [ "$image" != "$new" ] || return 1

  image="$new"
  echo "Displaying $image"
  cargo run -q -- "$image"
}

[[ $(type -t "get-image-$source") == "function" ]] || (echo "unknown album art source '$source'" && exit 1)

while true
do
  wait=30
  if update-info
  then
    log
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
