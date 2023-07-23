# username-checker

- small self-bot that checks if a username is available for use or not, if it is, it will sign it to the current user
- template at [gramme-rs template](https://github.com/MrAliSalehi/gramme-rs-template)


## use

- `git clone https://github.com/MrAliSalehi/rs-username-checker`
- update the `.env` and set the `API_HASH` & `API_ID` & `PHONE`
- `cargo build --release`
- `./rs-username-checker username`, note that the username should NOT include the `@`