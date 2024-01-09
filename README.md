# irc2torrent
...is primarily meant to be my training wheels for Rust.

It is a little app for use in Linux seedboxes mainly (it is cross-platform though) that you can use with TL initially.

The App connects to TL IRC server, joins "#tlannounces" channel and waits for the release announcements.

When there is a release, the App checks with your Regex entries in your options file and if it is a match, adds it to your rTorrent client defined in your options file. (Your rss key is also required)

Default configs are provided.
