A personal use converter from https://www.bilibili.com/video/online.html to RSS.

Because there are too many video authors I don't care about, so a blacklist is included.
This blacklist is highly personal.

Can be deployed on local or Replit, read with NetNewsWire.
(Need to add pkgs.openssl and pkgs.pkg-config to Replit's nix deps)
Sample site: https://bilibili-online-filtered-rss.zhangstef.repl.co


# APIs
- `GET host:port` get rss content
- `GET host:port/blacklist` get blacklist
- `PATCH host:port/blacklist` with json array of strings to add new items to blacklist
- `PUT host:port/blacklist` with json array of strings to replace blacklist
