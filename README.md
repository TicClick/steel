# steel

## what?

`steel` is a text chat client tailored to osu!, which provides the following features below the bare minimum ("send and receive messages"):

- custom, trackable chat highlights
- favourite channel list
- a Google Translate shortcut on chat messages
- a slightly customizable UI palette
- rudimentary plugin system (shit may break, but plugin-breaking changes will be indicated)

## where?

[download the latest version](https://github.com/TicClick/steel/releases/latest) and extract it into a separate folder. enable automatic updates for better experience.

## FAQ

### so uhh what does it look like?

[see for yourself](media/github-assets/main-window.png).

### I found an issue!

https://github.com/TicClick/steel/issues is the place.

### is it cross-platform?

yes, with Windows/Linux/MacOS support. only Windows is thoroughly tested, so watch out!

### what's the chat transport?

right now, it's the [IRC](https://osu.ppy.sh/wiki/IRC) gateway, but hopefully the app can be migrated to the websocket API once https://github.com/ppy/osu-web/issues/10118 is resolved.

### will you know my password?

no: it's stored locally, and only sent to the chat server -- see the source code, or listen to the app's network activity. two things to make a note of, though:

- it may be not safe from the plugins -- "install" them only if you trust their developers.
- since the IRC server doesn't support SSL, the password is sent in CLEAR TEXT -- if someone is spying on the network, they will be able to eavesdrop and take it.
  - on the other hand, if someone is spying on you, an exposed osu! **chat** password is one of the least concerns..

### plugins?

yeah, the app is to a certain degree extensible -- it's possible to make a Rust dynamic library using the interface from [`steel_plugin`](https://github.com/TicClick/steel/tree/master/crates/steel_plugin). please note that the dependencies are non-ABI safe -- as mentioned before, shit may break (either by itself, or by not yours truly).

still, something can definitely be hacked together!
