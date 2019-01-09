# mailbox

> ✉️ 💌 ✉️ imap query client that speaks json

## 👩‍🏭 development

This is a [rustlang](https://www.rust-lang.org/en-US/) application.
Go grab yourself a copy with [rustup](https://rustup.rs/).

## 🤸 usage

The default imap domain is Google's. 

It is recommended to pass in your [Mail app password](https://support.google.com/accounts/answer/185833?hl=en) from stdin.

```sh
echo -e "apppassword" | cargo run -p mailbox-cli -- -u "you@gmail.com" subject:lotto
```

Meetup, Inc 2019