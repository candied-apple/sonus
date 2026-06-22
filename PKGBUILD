# Maintainer: candiedapple <alperenger@gmail.com>
pkgname=sonus
pkgver=0.2.4
pkgrel=1
pkgdesc="Terminal music player for YouTube Music"
arch=('x86_64')
url="https://github.com/candied-apple/sonus"
license=('MIT')
depends=('yt-dlp' 'alsa-lib')
makedepends=('cargo')
source=("git+https://github.com/candied-apple/sonus.git#tag=v${pkgver}")
sha256sums=('SKIP')
options=(!lto)

build() {
  cd "$pkgname"
  export LDFLAGS=""
  cargo build --release
}

package() {
  cd "$pkgname"
  install -Dm755 "target/release/sonus" "$pkgdir/usr/bin/sonus"
}
