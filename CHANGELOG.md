# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [v0.3.0] - 2021-04-16
 - rework fan-out filtering to include negated label values and empty labels
 - another dependency bump

## [v0.2.1] - 2021-04-05
 - dependency bump

## [v0.2.0] - 2021-03-31
 - drop labels with illegal keys according to prometheus' data model.
 - template targets using tera
 - add pagination support
 - add container image
 - set log level in config file
 - pretty print json for better debuggability
 - fan-out filtering: create multiple sd files based on labels

## [v0.1.0] - 2020-06-10
Initial release.
