# The PHP oracle: the PHP version the recorded vectors come from, with
# pdo_sqlite for the Connection recorder. Recording and comparison only;
# never deployed.
FROM php:8.4.27-cli-bookworm

RUN apt-get update \
    && apt-get install -y --no-install-recommends libicu-dev libsqlite3-dev \
    && docker-php-ext-install intl pdo_sqlite \
    && rm -rf /var/lib/apt/lists/*

# /work is this repository (read-only); recorders load doctrine/dbal from
# /work/upstream/doctrine/<package> (scripts/fetch-upstream.sh).
WORKDIR /work
ENTRYPOINT ["php", "-d", "display_errors=stderr"]
