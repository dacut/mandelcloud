#!/bin/bash -ex
ENVIRONMENT=dev
TAG=$(date -u +%Y%m%dT%H%M%SZ)

main () {
    while [[ $# -gt 0 ]]; do
        case $1 in
            --environment | -e )
                if [[ $# -lt 2 ]]; then
                    echo "$1 requires an argument"
                    usage;
                    exit 1
                fi
                ENVIRONMENT=$2;
                shift 2;;
            
            --environment=* )
                ENVIRONMENT="${1#*=}"
                shift;;

            --tag | -t )
                if [[ $# -lt 2 ]]; then
                    echo "$1 requires an argument"
                    usage;
                    exit 1
                fi
                TAG=$2;
                shift 2;;
            
            --tag=* )
                TAG="${1#*=}"
                shift;;

            -- )
                shift;
                break;;

            --* | -* )
                echo "Unknown argument: $1"
                exit 1;;

            * )
                break;;
        esac
    done

    if [[ $# -gt 0 ]]; then
        REPOSITORY_URL=$1;
        shift;
    fi

    if [[ $# -gt 0 ]]; then
        echo "Unknown argument: $1";
        usage;
        exit 1;
    fi

    DOCKER_ARGS="--file lambda-image.dockerfile --platform linux/arm64"

    if [[ ! -z "${REPOSITORY_URL}" ]]; then
        DOCKER_ARGS="${DOCKER_ARGS} --tag ${REPOSITORY_URL}:${TAG} --push"
    fi

    if [[ -z "${REPOSITORY_URL}" ]]; then
        docker build $DOCKER_ARGS .
    else
        REPOSITORY_BASE=$(echo ${REPOSITORY_URL} | cut -d '/' -f 1)
        docker logout ${REPOSITORY_BASE} || true
        aws ecr get-login-password | docker login --username AWS --password-stdin ${REPOSITORY_BASE}
        docker build $DOCKER_ARGS .
        aws ssm put-parameter --overwrite --name "/mandelcloud/${ENVIRONMENT}/lambda-image-url" --type "String" --value "${REPOSITORY_URL}:${TAG}"
    fi;
}

help() {
    exec 2>&1;
    usage
    exit 0
}

usage() {
    cat 1>&2 <<EOF
Usage: lambda-build.sh [OPTIONS] [<repository-url>]
Builds the lambda image and pushes it to a private AWS Elastic Container
Repository if specified.

If a repository URL is specified, the image URL is written to an AWS Systems
Manager Parameter Store parameter named
"/mandelcloud/\${ENVIRONMENT}/lambda-image-url".

Options:
    --environment=ENVIRONMENT, -e ENVIRONMENT
        The environment to build the image for. Defaults to "dev".

    --tag=TAG
        The tag to apply to the image. Defaults to the current date and time in
        the UTC time zone in compact ISO 8601 format (e.g. 20241101T000000Z).

Arguments:
    <repository-url>
        The URL of the repository to push the image to. If unspecified, the
        value of the REPOSITORY_URL environment variable is used. If that is
        also unspecified, a Docker image is not pushed.
EOF
}

eval main "$@"
