[Cucumber doc](https://cucumber-rs.github.io/cucumber/current/quickstart.html)

https://docs.rs/crate/reqwest/latest

[Reqwest](https://docs.rs/reqwest/latest/reqwest/)

[Log crate](https://docs.rs/log/latest/log/index.html)
- https://github.com/borntyping/rust-simple_logger

```bash
cargo test --test url_shortener_steps
```
----

spin up a docker image and tear it down before each scenario


clean containers
````bash
docker stop $(docker ps -a -q) && docker container prune
```