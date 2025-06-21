#!/bin/bash

set -euo pipefail

configure_zsh() {
	echo "Configuring zsh..."
	if ! grep -q "plugins=(.*rust.*)" "$HOME/.zshrc"; then
		echo "Adding rust plugin to zsh..."
		sed -i.bak 's/plugins=(/plugins=(rust /' "$HOME/.zshrc"
	fi
	echo "Done configuring zsh."
}

main(){
	configure_zsh
}

main "$@"