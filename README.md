# Gratitude Bot
## Add the bot to your server!
This is as easy as clicking
[here](https://discord.com/api/oauth2/authorize?client_id=1094831789442343002&permissions=1024&scope=applications.commands%20bot)!

## What is this bot?
This bot will randomly remind registered users to keep a
[gratitude journal](https://youtu.be/WPPPFqsECz0), right within Discord!  
It runs on
[Cloudflare workers](https://workers.dev/), and is very simple to use:
1. Use `/help` to get more information
1. Use `/start` to begin keeping your journal
1. Use `/entry` to add entries even if the bot didn't send you a reminder yet
1. Use `/stop` to stop receiving reminders

And that's it! New features will be added in the future, and I'm happy to receive
[suggestions](https://github.com/Fittiboy/gratitude/issues/new?assignees=&labels=&template=feature_request.md&title=Feature+request%21)!
I would be grateful for any bugs you
[report](https://github.com/Fittiboy/gratitude/issues/new?assignees=&labels=&template=bug_report.md&title=Bug+report%21) as well!

### Self-hosting
The bot should be able to run on the free Cloudflare Workers plan,
but your mileage may vary. Early in development, there were some
requests that exceeded the free plan's resources, but self-hosting
should work if you don't have many users.

# Special Thanks!
Big thank you to the author of [this wonderful template](https://github.com/mcdallas/rust-discord-bot),
which only needed very slight modification to get up and running.  
I did not expect that someone had already covered the extremely
niche use-case of a Cloudflare Workers Discord bot written in Rust,
but here we are!
