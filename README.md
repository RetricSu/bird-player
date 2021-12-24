# Music Player

A simple GUI music player inspired by foobar2000 written in Rust using [egui](https://github.com/emilk/egui).
The goal of this project is to learn about making gui/ native apps, audio, databases / text search.
It is not meant to be used as a serious audio player.

## Goals

- Basic music player functionality. Play, pause, stop.
- Create a music library, which is indexed for searching.
- Parse id3 tags from tracks for use with indexing.
- Create playlists, which can be saved, opened, edited, reordered
- Drag n' Drop tracks from the music library into the playlist.
- Save last state of the app when closing.

## Stretch goals

- See if I can make right-click context menus.
- Visualizations
- Stream audio
- Swappable frontend so I can try other Rust cross platform gui libaries.

## Stuff to fix or implement

- [x] Reference playlists by index or actual reference (not a clone...), so info is not lost when changing playlist context
- [ ] Double clicking track automatically starts to play it.
- [ ] Remove tracks from playlist.
- [ ] Reorder items in playlist.
- [ ] Save playlists.
- [ ] Add Next and Previous controls
- [ ] Pause is a toggle
- [ ] Play restarts the track
- [ ] Add player indicators next to the track
- [ ] Add volume control slider
- [ ] Save app state on close.
- [ ] Set currently playing track as app Title
- [ ] Stop with all the cloning... seriously. Everything is cloned.
- [ ] Handle files which can't be decoded correctly into audio. 
- [ ] Implement library and text search.
- [ ] Playlist plays to end after track is selected.
- [ ] Un-named playlists get `(idx)` appended 
- [ ] Playlist tab section stacks playlist tabs when they don't fit.
