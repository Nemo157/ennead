#!/usr/bin/env bash

set -euo pipefail

ennead-cli() {
  if [[ -n "${ENNEAD_DEV:-}" ]]; then
    cargo run -q --manifest-path cli/Cargo.toml -- "$@"
  else
    ἐννεάς-cli "$@"
  fi
}

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

kurl() {
  curl -s -H 'User-Agent: ἐννεάς (https://github.com/Nemo157/ennead)' "$@"
}

get-playing-now() {
  kurl "https://api.listenbrainz.org/1/user/$user/playing-now" | jq -rMc '.payload.listens[0].track_metadata.additional_info.release_mbid // ""'
}

info="$(load-cached-info)"
update-info() {
  local newmbid="$(get-playing-now)"
  [ -n "$newmbid" ] && [ "$(query .id)" != "$newmbid" ] || return 1
  info="$(kurl "https://musicbrainz.org/ws/2/release/$newmbid?inc=release-groups+artists&fmt=json")"
  save-cached-info
}

query() {
  jq -rMc "$1" <<<"$info"
}

get-image-beets() {
  local artist="$(query '.["artist-credit"] | map(.name + .joinphrase) | join("")')"
  local album="$(query .title)"

  [ -n "$artist" ] || return 1
  [ -n "$album" ] || return 1

  local image="$(beet list -a -f '$artpath' "albumartists::^$artist\$" "album::^$album\$")"

  [ -n "$image" ] || return 1

  echo "$image"
}

download-release-art() {
  local file="$1"

  local mbid="$(query .id)"

  [ -n "$mbid" ] || return 1

  kurl -fLo "$file" "https://coverartarchive.org/release/$mbid/front"
}

download-release-group-art() {
  local file="$1"

  local groupmbid="$(query '.["release-group"].id')"

  [ -n "$groupmbid" ] || return 1

  kurl -fLo "$file" "https://coverartarchive.org/release-group/$groupmbid/front"
}

get-image-coverartarchive() {
  local mbid="$(query .id)"
  local file="$cache_dir/$mbid.cover.image" # unknown image type, use arbitrary suffix

  [ -n "$mbid" ] || return 1

  if ! [[ -f "$file" ]]
  then
    echo >&2 "downloading cover art"
    if ! (download-release-art "$file" || download-release-group-art "$file")
    then
      echo >&2 "download failed"
      return 1
    fi
  fi

  echo "$file"
}

log() {
  local artist="$(query '.["artist-credit"] | map(.name + .joinphrase) | join("")')"
  local album="$(query .title)"

  echo >&2 "Listening to $artist - $album"
}

image=
change-image() {
  local new="$("get-image-$source")"

  [ -n "$new" ] && [ "$image" != "$new" ] || return 1

  image="$new"
  echo >&2 "Displaying $image"
  ennead-cli --dither atkinson --scale fit "$image"
}

[[ $(type -t "get-image-$source") == "function" ]] || (echo >&2 "unknown album art source '$source'" && exit 1)

interactive=false
[[ -t 2 ]] && interactive=true

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

  if $interactive
  then
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
  else
    sleep "$wait"
  fi
done
