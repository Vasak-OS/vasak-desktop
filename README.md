# Vasak Desktop

[![Quality Gate Status](https://sonarcloud.io/api/project_badges/measure?project=Vasak-OS_vasak-desktop&metric=alert_status)](https://sonarcloud.io/summary/new_code?id=Vasak-OS_vasak-desktop)

Vasak is a desktop environment for Linux. It is designed to be simple, fast, and easy to use. It is written in JavaScript and uses HTML. It is currently in development. The goal is to have a desktop environment that is simple to use, but still has all the features you need.

## Dependencies

Vasak Desktop depends on the following packages:

* [Bootstrap](https://getbootstrap.com/) (bootstrap)
* [NW.js](https://nwjs.io/) (nwjs)
* nwjs-ffmpeg-codecs
* [Vue](https://vuejs.org/) (libvasak-vue)
* libvasak-ui
* vasak-desktop-service
* vasak-desktop-settings
* navale

### Build Dependencies

* [Node.js](https://nodejs.org/)
* [NPM](https://www.npmjs.com/)

## Start Vasak Desktop

To start Vasak Desktop, run the following steps:

1. Clone the repository

```bash
git clone git@github.com:Vasak-OS/vasak-desktop.git
```

2. Move to the directory

```bash
cd vasak-desktop
```

3. Install dependencies

```bash
npm install
```

4. Start Vasak Desktop

```bash
nw ./
```

## Build Vasak Desktop

[PKGBUILD](https://github.com/Vasak-OS/PKGBUILDS/blob/main/vasak-desktop/PKGBUILD)

## Contributing

Pull requests are welcome. For major changes, please open an issue first to discuss what you would like to change. Please make sure to update tests as appropriate.

1. Fork it
2. Create your feature branch

```bash
git checkout -b feature/battery-indicator
```

3. Commit your changes 

```bash
git commit -am 'Add some battery-indicator'
```

4. Push to the branch

```bash
git push origin feature/battery-indicator
```

5. Create a new Pull Request

## Acknowledgements

- [Bootstrap](https://getbootstrap.com/)
- [Vue](https://vuejs.org/)
- [NW.js](https://nwjs.io/)
