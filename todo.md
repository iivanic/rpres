# rpres
- simple cli tool that open specified .md and then plays it as presentation in browser
- uses clap, tiny_http
- slides are splitted by headers:
- Title slide is # header
- Other pages are ## headers
- --click or -c or 'a' key(toggle) in browser turns on anim mode when every not empty line requires space or mouse click to be shown 
- --open or -o opens browser on localhst and correct port
- three types of html templates are embedded, can be listed and be chosen with --list-templates or -l and --template [name] or -t [name]
- use idiamatic rust project structure and code organization
- use some good .md to html crate
- add --paged or -p mode in which every slide is loaded from server with ajax. Defualt mode is to render whole presentation in html and use javascript to move between pages
- html templates are "terminal", "classic" and "modern"
- add param --server [ip address] or -s [ip address]
- create tests
- create test presentation with full .md feature set

