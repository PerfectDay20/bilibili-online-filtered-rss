An RSS converter of https://www.bilibili.com/video/online.html and other sites for personal use.

Because there are too many video authors and categories I don't care about, so a blacklist is included.
This blacklist is highly personal.

Can be deployed on local or Replit, read with NetNewsWire.
(Need to add pkgs.openssl and pkgs.pkg-config to Replit's nix deps)
Sample site: https://bilibili-online-filtered-rss.zhangstef.repl.co


# APIs
## BiliBili 
- `GET /` get rss content
- `GET /blacklist` get blacklist
- `PATCH /blacklist` with json blacklist body to add new items to blacklist, return the result blacklist
- `PUT /blacklist` with json blacklist body to replace blacklist, return the result blacklist


HTTP blacklist request body should be a json object, available fields are:
```json
{
    "authors": [
        "foo"
    ],
    "categories": [
        "bar"
    ]
}

```
The authors and categories can be got from the rss content.


## ddys.site
- `GET /ddys` get rss content of ddys


# CLI
```
Usage: bilibili-online-rss [OPTIONS]

Options:
      --host <HOST>            [default: 127.0.0.1]
  -p, --port <PORT>            [default: 3000]
  -b, --blacklist-path <FILE>  
  -h, --help                   Print help
  -V, --version                Print version

```
