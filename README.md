このプロジェクトは4/11をもって凍結されます。
archivedとして残す為、必要ならばforkして開発を進めてください。

herokuでRust・serenityのdiscordbotを動かせることを確認するためのプロジェクトです<br>
herokuのBuildpacksにhttps://github.com/emk/heroku-buildpack-rust.git を追加してあります<br>
cf. https://blog.ichyo.jp/posts/deploying-rust-applications-to-heroku/<br>

## 機能
環境変数"discord_bot_token"によってログインし、起動直後にbotのオーナーにDMを送ります。<br>
