# spotify-tg-bio

Обновляет био в телеграме текущим треком из спотифая. Сделано по просьбе @omar1ck.

Скорее всего, тг снесёт учётку за кучу обновлений :D

## Сборка

Надо подгрузить переменные окружения `SPOTIFY_CLIENT_ID` и `SPOTIFY_CLIENT_SECRET`. Ну а потом можно собирать:

```
cargo build --release
```

## Запуск

При первом запуске создаётся файл `config.json` с настройками. Нужно получить `app_id` и `app_hash` для доступа к Telegram API [тут](https://my.telegram.org/).

`interval_secs` - интервал обновления в секундах.

`bio_template` - шаблон био. `{artist}`, `{title}`, `{progress}`, `{duration}` будут заменены на соответствующие значения.
