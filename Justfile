version := `cargo get package.version`

no-exist:
    #!/usr/bin/env bash
    ! docker manifest inspect localhost:50225/mdingress:{{ version }}

build:
    docker buildx build --platform linux/amd64,linux/arm64 . --tag localhost:50225/mdingress:{{ version }}

publish: no-exist build
    docker push localhost:50225/mdingress:{{ version }}
