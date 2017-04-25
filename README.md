# D'Oh [![Build status](https://travis-ci.org/thecoshman/doh.svg)](https://travis-ci.org/thecoshman/doh) [![Licence](https://img.shields.io/badge/license-MIT-blue.svg?style=flat)](LICENSE) [![Crates.io version](http://meritbadge.herokuapp.com/doh)](https://crates.io/crates/doh)
D'Oh - Directories Over HTTP

CURL is great, it's probably what you want. There is a wee tiny chance though that you don't like the interface it offers, all the ugly html.
Let's say you used [`http`](https://github.com/thecoshman/http) (not to be confused with [HTTP](https://en.wikipedia.org/wiki/Hypertext_Transfer_Protocol))
and are hosting a folder that you can view in a browser, but want to view that folder from a command line, say hello to D'Oh!
It provides Directories Over HTTP.

This might be:

  * like an FTP style interface, or it could maybe do
  * some sort of mounted folder thing, or
  * something else.

Whatever it does, it'll probably be aimed at the server side being `http` (the program, not the protocol, well, also the protocol, but specfically the program).
