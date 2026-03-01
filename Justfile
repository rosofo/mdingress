version := `cargo get package.version`

build:
    docker buildx build --platform linux/amd64,linux/arm64 . --tag localhost:50225/mdingress:{{ version }}

publish: build
    docker push localhost:50225/mdingress:{{ version }}
