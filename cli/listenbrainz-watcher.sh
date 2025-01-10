#!/usr/bin/env bash

set -euo pipefail

user="${1:?missing listenbrainz username}"

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


info=
update-info() {
  if new="$(get-info)" && [ "$info" != "$new" ]
  then
    info="$new"
    return 0
  else
    return 1
  fi
}

query() {
  jq -rMc "$1" <<<"$info"
}

log() {
  echo "Listening to $(query .artist) - $(query .release)"
}

image=
change-image() {
  new="$(beet list -a -f '$artpath' "albumartists:$(query .artist)" "album:$(query .release)")"
  if [ -z "$new" ]
  then
    echo "Could not find album art"
    return 1
  fi

  if [ "$image" != "$new" ]
  then
    image="$new"
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
      break
    fi
    printf '\b \b'
  done
done
