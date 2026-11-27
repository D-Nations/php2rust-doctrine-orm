# php2rust-doctrine-orm

An unofficial Rust port of [Doctrine ORM](https://github.com/doctrine/orm)
3.7.5 and the Doctrine packages it needs (collections, lexer, persistence,
event-manager, inflector, instantiator), behaviour for behaviour. The
upstream PHPUnit tests are ported with the same names, and vectors recorded
from real PHP 8.4.27 pin what the tests leave open. Not affiliated with or
endorsed by the Doctrine Project.

Part of the php2rust project. DBAL comes from
[php2rust/doctrine-dbal-rs](https://github.com/php2rust/doctrine-dbal-rs)
and PHP's own behaviour (values, arrays, dates) from
[php2rust/compatibility-tools](https://github.com/php2rust/compatibility-tools),
both as git dependencies pinned to a commit.

**Status.** Work in progress. One crate per package in `crates/`; each
crate's README has its contract table, and `parity/doctrine/<package>.tsv`
lists every upstream test as `ported`, `n/a` or `pending`. Not published to
crates.io (`publish = false`).

To change a git dependency and see the result here at once, add an
uncommitted `.cargo/config.toml` with a `[patch."<git url>"]` section that
points to your local checkout.

## Checks

There is no hosted CI. Run the checks locally:

```sh
bash scripts/fetch-upstream.sh       # once: the pinned upstream sources
python scripts/dev_check.py full     # fmt, clippy -D warnings, tests, parity
```

## Licence

MIT, see [LICENSE](LICENSE). The Doctrine packages' source and tests are
copyright the Doctrine Project (MIT); the port keeps their notice.
