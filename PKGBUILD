# Maintainer: TTGTiSO-Desk Team <actions@github.com>
pkgname=ttgtiso-desk
pkgver=0.1.10
pkgrel=1
pkgdesc="TTGTiSO-Desk Remote Desktop Client and Server Agent"
arch=('x86_64')
url="https://github.com/uiper123/nxDesk2.0"
license=('MIT')
depends=('webkit2gtk-4.1' 'gtk3' 'cairo' 'pango' 'glib2' 'openssl' 'gdk-pixbuf2' 'libsoup3' 'alsa-lib')
makedepends=('cargo' 'npm' 'nodejs' 'git')
source=("$pkgname-$pkgver.tar.gz::https://github.com/uiper123/nxDesk2.0/archive/refs/tags/v$pkgver.tar.gz")
sha256sums=('SKIP')

build() {
    cd "nxDesk2.0-$pkgver"
    
    echo "Building Server Agent..."
    cargo build --release --package server-agent

    echo "Building API Server..."
    cargo build --release --package api-server
    
    echo "Building Desktop Client..."
    cd apps/desktop-client
    npm install
    npx tauri build --no-bundle
}

package() {
    cd "nxDesk2.0-$pkgver"
    
    # Install server agent
    install -Dm755 target/release/server-agent "$pkgdir/usr/bin/ttgtiso-desk-agent"
    
    # Install API server
    install -Dm755 target/release/api-server "$pkgdir/usr/bin/ttgtiso-desk-api"
    
    # Install desktop client binary
    install -Dm755 apps/desktop-client/src-tauri/target/release/appsdesktop-client "$pkgdir/usr/bin/ttgtiso-desk"
    
    # Install systemd service for agent (optional but good practice)
    # install -Dm644 packaging/systemd/ttgtiso-desk-agent.service "$pkgdir/usr/lib/systemd/system/ttgtiso-desk-agent.service"
}
