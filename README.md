# steel

## what?

`steel` is a text chat client tailored to osu!, which provides the following features below the bare minimum ("send and receive messages"):

- custom, trackable chat highlights
- favourite channel list
- a Google Translate shortcut on chat messages
- a slightly customizable UI palette

## where?

[download the latest version](https://github.com/TicClick/steel/releases/latest) and extract it into a separate folder. enable automatic updates for better experience.

## FAQ

### so uhh what does it look like?

[see for yourself](media/github-assets/main-window.png).

### is it cross-platform?

yes, with Windows/Linux/MacOS support. only Windows is thoroughly tested, so watch out!

### is it malware? my antivirus says so

one of the heuristics is [probably overly cautious](https://www.elevenforum.com/t/wacatac-h-ml-found-by-microsoft-defender-but-not-anything-else.13702/), since it can't verify who built the executable. whitelist the application, and both of you should be fine ([see example for Windows Defender](media/github-assets/whitelist-guide.png) -- also note the `!ml` suffix, which means "machine learning").

### I found an issue!

https://github.com/TicClick/steel/issues is the place.

### what's the chat transport?

right now, it's the [IRC](https://osu.ppy.sh/wiki/IRC) gateway, but hopefully the app can be migrated to the websocket API once https://github.com/ppy/osu-web/issues/10118 is resolved.

### will you know my password?

no: it's stored locally, and only sent to the chat server -- see the source code, or listen to the app's network activity. one to make a note of, though:

- since the IRC server doesn't support SSL, the password is sent in CLEAR TEXT -- if someone is spying on the network, they will be able to eavesdrop and take it.
  - on the other hand, if someone is spying on you, an exposed osu! **chat** password is one of the least concerns..
