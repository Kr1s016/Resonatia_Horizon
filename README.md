# Resonantia

Нативный trait-мод для [Horizon](https://github.com/horizon-rust/horizon): всё в мире звучит
восемью тонами. Слушай вещи камертоном, копи прозрение и настраивайся на новые устройства.
Дизайн и план — в [DESIGN.md](DESIGN.md).

## Состав

```text
horizon-mixin.toml   бандл: [mod], пин Horizon, [[crate]] core, [api] api
horizon-mod.toml     контентная часть (предметы, модели, языки)
core/                правила без игровых типов: тоны, профили, знание, настройки (+ тесты)
api/                 связка с игрой: хуки Hooks, attachment знания, команды
data/, assets/       объявления предметов, модели, en_us / ru_ru
```

## Проверка

```sh
cd core && cargo test
```

## Сборка в игру

```sh
horizon-launcher native-build /path/to/resonantia --instance <instance> --trust-native-code
```

Мод запинен на коммит Horizon `1511bd7a23992d219e8d449f26d140bfcdd1434e`. В игре:
ПКМ камертоном по блоку, `/resonantia`, `/resonantia attune first_chord`.

Пока камертон и кодекс используют ванильные модели аметистового осколка и книги:
своих текстур ещё нет.
