# Deliverust

Deliverust is a pub-sub mechanism for a lightweight and easy to use Actor
model in rust.
This is neat for complex project will multiple components that handle simple tasks.
Currently this is not designed for high stream of data, although the `ModuleTable`
should be able to take a few hits

## What's inside

* The `Subscriber` trait is the entrypoint of the pub-sub mechanism.
  Any type can publish directly any message, but one type can only handle type
  it have subscribed to.
* The `ModuleTable` hide all the magic, notably with usage of callbacks and `Any`.

## What's next

* Metrics of throughput in different modules
* Maybe the channel mechanism will be delegated to the crate using proc-macros
* Security of usage, notably to avoid deadlocks

## How to contribute

* Just open an MR or issues, I'll gladly review them in an human acceptable delay.
