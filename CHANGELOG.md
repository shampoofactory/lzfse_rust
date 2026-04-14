
# Change Log
All notable changes to this project will be documented in this file.
 
The format is based on [Keep a Changelog](http://keepachangelog.com/)
and this project adheres to [Semantic Versioning](http://semver.org/).
 
## [Unreleased] - yyyy-mm-dd
  
### Added

### Changed

### Fixed


## [0.2.1] - 2026-04-14

### Changed
- ShortBytes reworked with pointers.

### Fixed
- BitDist undefined behavior (Miri).
- Ring/ RingView formally expose full buffer bounds (Miri).


## [0.2.0] - 2022-03-14
   
### Added
- Additional code tests.
- X86 BMI2 optimized bit masks.

### Changed
- BitReader simplified and optimized.
- FrontendBytes i32::MAX limit removed.
- Fse Decoder VEntry table flattened (simplifies pointer arithmetic).

### Fixed
- Bash scripts misc fixes.
- Clippy warnings.
- Docs clarifications, grammar and typo fixes.
- LzfseEncoder erroneous RAW encoding selection: https://github.com/shampoofactory/lzfse_rust/issues/1


## [0.1.0] - 2021-04-22 [Yanked]
 
Initial release.
