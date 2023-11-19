# integral

# Support Platforms
- Library (Usable in other rust proyects)
- Web (Wasm)

# Support Stores- GithubIO (only for web)# Requirements
- Rust
- Cargo
- [Cargo Generate](https://github.com/cargo-generate/cargo-generate)
- [Cargo Make](https://github.com/sagiegurari/cargo-make) (Optional)
- [Cargo Release](https://github.com/crate-ci/cargo-release) (Optional)
- [Trunk](https://trunkrs.dev) (Optional for web development)

# Configure Github Actions
> [!WARNING]
> After initializing this project and having activated the github workflows, you need to configure the secret variables in your github project (This is done this way to protect the security of your data).

- CI: **Not required**
- CD/Deployment:**Not required**

# Development Guide
> [!NOTE]
> If you want generate all needed for icons, you can use [this page](https://icon.kitchen)
- Edit the `.env` file if you need
- Edit `src` folder
- Run `cargo make dev` for run as development mode
- Run `cargo make --list-all-steps` for check all aviable tasks
- To upload a new version and trigger all the workflows related to the deployment of a new version, you just have to run the command `cargo release -x patch` (See the `cargo release -h` for more information)

## Other CargoMake Tasks
* **check** - Check all issues, format and code quality
* **clean** - Clean all target directory
* **clippy** - Check code quality
* **default** - Check all issues, format and code quality
* **dev** - Run native launcher with development configuration
* **fix-all** - Try fix all clippy and format issues
* **fix-fmt** - Fix format
* **fmt** - Check format quality
* **test** - Check all unit test
