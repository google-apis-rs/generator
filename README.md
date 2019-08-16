# Overview

 * discovery_parser: The most fundamental translation of the google discovery
   doscument into rust types.
 * field_selector/field_selector_derive: Includes the FieldSelector trait and a
   proc-macro to automatically derive the trait from a struct. FieldSelector
   provides a method to return a value to be used in the `fields` attribute of
   google apis. Syntax would look something like
   `kind,items(title,characteristics/length)`
 * uri_template_parser: Parse uri templates (RFC 6570) into an AST. The AST can
   represent the full syntax as described in the RFC, but does not attempt to
   produce the rendered template output. The AST is used by the generator to
   generate rust code that renders the template.
 * generator: This is the primary purpose of this repository. Given a discovery
   document it will produce idiomatic rust bindings to work with the API. The
   input is a discovery document and the output is rust crate at a specified
   directory.

# Community

* **[Kanban Board](https://github.com/google-apis-rs/apis/projects/1)**
   * Learn what is currently being worked on, and what's in the backlog

* **[Chat](https://gitter.im/google-apis-rs/community)**
   * Join and let us know what you think :)!
   * Talk to the devs, or see what they are talking about.

# Development

[![Build Status](https://travis-ci.org/google-apis-rs/apis.svg?branch=master)](https://travis-ci.org/google-apis-rs/apis)

* `cargo run -- --help`
   * Run the master control program and see what it can do

* `make`
   * See which tasks you can perform using make

# 🛸Project Goals🛸

These are snatched from the original project (_OP_), with some adjustments and amendments.
Even though the list is ordered, they are not (yet) ordered by priority, but merely to make each point adressable, as in `🛸2`.

1. provide an idiomatic rust implementation for Google APIs, which includes _type safety_ and native _async_ operations.
2. first-class documentation with cross-links and complete code-examples
3. support all API features, including downloads and resumable uploads
4. Convenient CLIs are provided on top of the API for use in shell scripts
5. API and CLI generation can be customized easily to overcome issues with your particular API
   * **Byron thinks that** we cannot assume to get it right for all APIs from the start unless we actually test everything ourselves. Thus if we enable people to help themselves, it will help us further down the line.
6. Built-in debugging and tracing allows to understand what's going on, and what's wrong.
   * **Byron thinks that** providing full output of requests and responses when using the CLI really helped. However, this time around there might be some more logging, using `tracing` or `log` at least. Here once again it becomes interesting to see if different systems can be supported, to allow people to tailor their experience based on their needs. `cargo features` could come a long way.
7. The code we generate defines the standard for interacting with Google services via Rust.
   * Google uses these crates! They are that good! 😉 (Google uses more efficient means internally, no JSON involved!)
8. The code base is made for accepting PRs and making contributions easy
   * To stay relevant, people must contribute.
   * The original authors won't stay around forever (see [GitPython](https://github.com/gitpython-developers/GitPython))
9. _safety and resilience_ are built-in, allowing you to create highly available tools on top of it. For example, you can trigger retries for all operations that may temporarily fail, e.g. due to network outage.
   * **Byron thinks that** this could be naturally supported by the async ecosystem, and thus helps slim down the delegate.

# Learning from the past

Let's keep in mind what worked and what didn't.

## 🌈What worked well in _OP_🌈

1. **`make` to track dependencies and drive tooling**
   * Having built-in help was nice, and one go-to location to drive everything around the project
1. **Building big data models**
   * It was very helpful to have all data available in a tree
   * Merging structured data into even bigger trees helped to keep all data in easy to edit, human readable files, even with the option to pull in 'override files' to patch API descriptions as needed. The latter was used with the [Drive API](https://github.com/Byron/google-apis-rs/blob/master/etc/api/drive/v2/drive-api_overrides.yaml#L1), even though I would only add such capability on an as-needed basis.
1. **Having an off-the-shelf template engine**
   * Generating text from 'smart' files with some basic support for syntax highlighting made it quite easy to get started.
   * This was only possible through the `make` driven automation, as one would have to run multiple tools including `cargo check` to
    see if it actually worked.
1. **Having as much code as possible available without template**
   * particularly including `util.rs` helped to provide common code, and I would put as much code as possible into Rust-only files.
   * code size could further be reduced by putting that code into its own crate and maintain it as such.
1. **Performance**
   * Code generation was fast enough and could be parallelized on a per-API/CLI basis thanks to `make`.
1. **API, Docs, and CLIs**
   * I think having fully type-safe and chainable APIs was great.
   * The docs were lovely
   * The CLIs allowed to use an API almost instantly

## 🥵Issues with _OP_'s way of doing things🥵

1. **logic in templates**
   * What seemed like a huge benefit was also causing vastly difficult to read and understand templates.
   * I remember that sometimes, I worked around certain limitations of the engine, which masked what one was actually doing.
   * Separation of concerns is a good thing, but _OP_ didn't have it. The template engine transformed the data model from _discovery_
    on the fly.
1. **massive code duplication caused huge libraries**
   * Each call would be 'spelled out', and APIs came with many calls. This caused massive libraries that take a while to check and build.
   * Huge files are harder to read
1. **improper namespace handling**
   * types introduced by the API could clash with reserved names of the Rust language, or with names imported into the namespace.
   * It wasn't easy to handle names consistently in all places that needed them
1. **python and mako**
   * Even though they worked, it was another thing that had to be installed by `make`, and could just fail [for some](https://github.com/Byron/google-apis-rs/issues/234)
1. **arbitrary smartness**
   * In order to fix issues with the type system and make numbers more easily usable by converting "strings" into integers/floats, what worked in one API broke another.
1. **artificial strupidity when dealing with dates and time**
   * as opposed to trying to be smart with numbers, we were not converting the uniformly represented date formats into something like `chrono::*`.
1. **it's cumbersome to actually use a CLI**
   * Even though authentication was dealt with nicely for the most part, actually using APIs required them to be enabled via the developer console. From there one would download a file and deposit it in the right spot. Only then one could start using the CLI.
1. **oddly keyed login tokens stored on disk per scope**
   * due to tokens being hashed by the scope(s) they represent, choosing a different scope forced you to re-authenticate, even though another token file already included the scope you wanted.
1. **it took at least 6 weeks to get the first release on crates.io**
   * development wasn't the fastest, and I claim one was slowed down due to too much manual testing.
1. **there was no way to use the CI to test CLIs to actually interact with Google APIs**
   * This was due to API usage being bound to a person, and these credentials were nothing you would want to have lying around in a public git repository.
   * Not being able to test certain feature automatically and repeatedly takes time and reduces quality guarantees.
1. **tests were minimal and most testing was like "if it compiles, it's good"**
   * The _OP_ suffered from only having a few tests, and even though Rust only compiles decent quality software, by nature, certain reasoning was just in my head. _'Why is it doing this particular thing? It seems wrong'_ would be impossible to know. 
   * manual testing is slow and error prone
1. **Versions like 1.2.0+2019-08-23...` would additionally show the version of the Google API description, but is ignored by cargo`** 
   * This was done to discriminate the 'code' version from the version of the API it represents.
   * As the `+` is ignored by `cargo`, to re-release a crate with a new version of the API, one would have to increment the patch level. However, that would force all crates to be re-released, even if their API version didn't change at all.
   * This caused unnecessary spamming of `crates.io`, and the `+` should be a `-` to fix this.
1. **The 'fields' parameter could not be used to have smaller response types**
   * Some [Response Types](https://docs.rs/google-sheets4/1.0.10+20190625/google_sheets4/struct.Response.html) are huge, even though with the right `field` setting, one would only receive a fraction of the data. However, one would always have to allocate big structures to with most of the optional fields set to `None`.
   * 💡Idea 💡: Can [serde(flatten)](https://serde.rs/field-attrs.html#flatten) be used to subdivide the possible field sets in the data structure? Probably it's not known which actual fields belong to each `field` argument.


# Technology and Architecture Sketches

Items mentioned below ideally create a link to one of the problems they slove, e.g. `🥵2` , the project goal they support, e.g `🛸3`, or the effective thing they build on (`🌈1`).

## Toolchains

Here is the anticipated tooling. What follows is the list of tools I would add and why.

* **make** - repeats `🌈1` 
  * I am a fan of simple makefiles, which catch dependencies between files and run a script to generate them. This served _OP_ extremely well.
  * get parallelization for free, and make transparent which programs to call and how to get work done.
  * the Makefile serves as hub keeping all commands one would run to interact with the project in any way.
  * It helps to generate crates only when needed, and can help manage publishing of crates while avoiding trying to upload duplicates.
* **Cargo/Rust** - fixes `🥵4`, support `🌈1`
  * All work should be done by a Rust binary, which helps keeping things easy with `make`. Previously, the single, magical binary was `python`, and when adding even a single additional Rust tool, one would pay with some complexity. One Rust binary with multiple sub-command seems reasonable.
* **rust-fmt** - helps remedy `🥵8`
  * Definitely needed to get idiomatically looking code.
  * _OP_ didn't have it, it wasn't a real problem, but too much time was spent making things look pretty. With `rust-fmt`, templates can be optimized for maintainability, even if the output doesn't look great initially.

# Development Goals

These should optimize for allowing a pleasant developer experience, at the beginning of the project as well as things stabilize. They should support the project goals or at least not hinder them. For example, settings things up in a way that is hard to use to the average person would be in the way of allowing folks to 'easily' fix issues they encounter.

* **TDD** (supports )
  * Everything done should be driven by at least one test which can be run automatically.
  * **Byron also thinks that** this is totally doable without breaking into sweat.
* **Journey Testing**
    
