run:
    cargo run --features bevy/dynamic_linking --target-dir target-desktop

watch:
    # in case target dir/dist if doesn't exist
    mkdir -p target-desktop
    trunk watch -i.pijul -itarget-desktop --enable-cooldown

serve:
    mkdir -p dist
    simple-http-server dist

build-release:
    cargo build --release --target-dir target-desktop

zellij:
    zellij -l dev.kdl a -cf acelr-game-session

kill:
    zellij d -f acelr-game-session


build-web:
    trunk build
    
publish-web: build-web && itch

publish-web-release: && itch
    trunk build --release

itch:
    butler push dist Jengamon/just-a-number-fcfi:html5
