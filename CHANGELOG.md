# Changelog

## [Unreleased]
### Changed
* Improved listing of projects
    * By default projects with no active todos will be filtered out. This can be
        disabled by specifying the `-i` filter which will lead to all projects
        being printed regardless of active todos or not.
    * Now using [prettytable-rs](https://github.com/phsym/prettytable-rs) to
        list projects.
    * Additionaly print out the active, done and total todos for the project.
