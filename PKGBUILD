# Maintainer: Your Name <you@example.com>
pkgname=sysevents
pkgver=1.0.0
pkgrel=1
pkgdesc="Show boot, shutdown, suspend and resume events for a given date, read from the systemd journal"
arch=('x86_64' 'aarch64')
url="https://github.com/yourname/sysevents"
license=('MIT')
depends=('systemd' 'gcc-libs')
makedepends=('cargo')
checkdepends=('systemd')
options=('!lto') # LTO is already configured explicitly in Cargo.toml's release profile
source=("$pkgname-$pkgver.tar.gz::$url/archive/refs/tags/v$pkgver.tar.gz")
sha256sums=('REPLACE_WITH_REAL_SHA256_AFTER_TAGGING_RELEASE')

prepare() {
    cd "$pkgname-$pkgver"
    cargo fetch --locked --target "$(rustc -vV | sed -n 's/host: //p')"
}

build() {
    cd "$pkgname-$pkgver"
    export RUSTUP_TOOLCHAIN=stable
    export CARGO_TARGET_DIR=target
    cargo build --frozen --release --all-features
}

check() {
    cd "$pkgname-$pkgver"
    cargo test --frozen --release
}

package() {
    cd "$pkgname-$pkgver"
    install -Dm755 "target/release/$pkgname" "$pkgdir/usr/bin/$pkgname"
    install -Dm644 LICENSE "$pkgdir/usr/share/licenses/$pkgname/LICENSE"
    install -Dm644 README.md "$pkgdir/usr/share/doc/$pkgname/README.md"
}
