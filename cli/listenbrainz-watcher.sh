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

download-release-art() {
  local mbid="$1"
  local file="$2"

  curl -sfLo "$file" "https://coverartarchive.org/release/$mbid/front"
}

download-release-group-art() {
  local mbid="$1"
  local file="$2"

  local groupmbid="$(curl -s "https://musicbrainz.org/ws/2/release/$mbid?inc=release-groups&fmt=json" | jq -r '.["release-group"].id')"

  [ -n "$groupmbid" ] || return 1

  curl -sfLo "$file" "https://coverartarchive.org/release-group/$groupmbid/front"
}

get-image-coverartarchive() {
  local mbid="$(query .mbid)"
  local file="$cache_dir/$mbid.cover.jpg" # probably jpg, but maybe not, doesn't really matter

  [ -n "$mbid" ] || return 1

  if ! [[ -f "$file" ]]
  then
    echo >&2 "downloading cover art"
    if ! (download-release-art "$mbid" "$file" || download-release-group-art "$mbid" "$file")
    then
      echo >&2 "download failed"
      return 1
    fi
  fi

  echo "$file"
}

log() {
  local artist="$(query .artist)"
  local album="$(query .release)"

  echo >&2 "Listening to $artist - $album"
}

image=
change-image() {
  local new="$("get-image-$source")"

  [ -n "$new" ] && [ "$image" != "$new" ] || return 1

  image="$new"
  echo >&2 "Displaying $image"
  cargo run -q -- --dither atkinson --scale fit "$image"
}

[[ $(type -t "get-image-$source") == "function" ]] || (echo >&2 "unknown album art source '$source'" && exit 1)

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
    printf >&2 '·'
  done

  for (( i = wait; i > 0; i-- ))
  do
    if read -st 1
    then
      printf >&2 '\r'
      for (( ; i > 0; i-- ))
      do
        printf >&2 ' '
      done
      printf >&2 '\r'
      break
    fi
    printf >&2 '\b \b'
  done
done
