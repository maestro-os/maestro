# syntax=docker/dockerfile:1

# renovate: datasource=github-releases packageName=junegunn/fzf
ARG FZF_VERSION='0.61.1'
# renovate: datasource=github-releases packageName=hadolint/hadolint
ARG HADOLINT_VERSION='2.12.0'
ARG BINUTILS_VERSION='2.42'

###############################################
FROM mcr.microsoft.com/devcontainers/rust:1-1-bullseye@sha256:d9c118e810133b8e49a0b32fb89ceb2c9c02d4d0a63cf7a68e28d4068e21afee AS base
SHELL ["/bin/bash", "-euxo", "pipefail", "-c"]
ARG TARGETPLATFORM

RUN <<EOT
	if [ "${TARGETPLATFORM}" != 'linux/amd64' ] && [ "${TARGETPLATFORM}" != 'linux/arm64' ]; then
		echo "Platform ${TARGETPLATFORM} currently not supported!" >&2
		exit 2
	fi
EOT

###############################################
FROM base AS builder-binutils
ARG TARGETARCH
ARG BINUTILS_VERSION

RUN <<EOF
	apt-get update
	apt-get install -y --no-install-recommends \
		bison \
		flex \
		gcc \
		libgmp3-dev \
		libmpc-dev \
		libmpfr-dev \
		texinfo \
	;
	rm -rf /var/lib/apt/lists/*

	wget -qO /tmp/binutils.tar.gz "https://ftp.gnu.org/gnu/binutils/binutils-${BINUTILS_VERSION}.tar.gz"
	tar -xzf /tmp/binutils.tar.gz -C /tmp/
	mkdir /tmp/binutils-build
	cd /tmp/binutils-build
	"../binutils-${BINUTILS_VERSION}/configure" --target=x86_64-elf --prefix=/usr/local --disable-nls
	make "-j$(nproc)"
EOF


###############################################
FROM base AS build-tools-utils
ARG TARGETARCH
ARG HADOLINT_VERSION FZF_VERSION

# hadolint ignore=DL4006
RUN <<EOF
	mkdir -p /tmp/output/
	wget -qO /tmp/output/hadolint "https://github.com/hadolint/hadolint/releases/download/v${HADOLINT_VERSION}/hadolint-Linux-$(echo "${TARGETARCH}" | sed s/amd64/x86_64/)"
	chmod +x /tmp/output/hadolint
	/tmp/output/hadolint --version

	wget -qO /tmp/fzf.tar.gz "https://github.com/junegunn/fzf/releases/download/v${FZF_VERSION}/fzf-${FZF_VERSION}-linux_${TARGETARCH}.tar.gz"
	tar -xzf /tmp/fzf.tar.gz -C /tmp/output/ --no-same-owner
	rm /tmp/fzf.tar.gz
	chmod +x /tmp/output/fzf
	/tmp/output/fzf --version
EOF

###############################################
FROM base AS development-base
ARG TARGETPLATFORM

RUN <<EOF
	# Note: Avoid using apt too much to optimize build time. wget binaries in another stage when possible.
	apt-get update
	apt-get install -y --no-install-recommends \
		clang \
		fdisk \
		gdb \
		qemu \
		qemu-system-arm \
		qemu-system-i386 \
		xorriso \
	;
	if [ "${TARGETPLATFORM}" = 'linux/amd64' ]; then
		apt-get install -y --no-install-recommends \
			gcc-multilib \
			grub \
			grub-pc-bin \
		;
	fi
	if [ "${TARGETPLATFORM}" = 'linux/arm64' ]; then
		:
	fi

	# ln -s /usr/bin/x86_64-linux-gnu-ld /usr/local/bin/x86_64-elf-ld
	rm -rf /var/lib/apt/lists/*
EOF

# apt-get install -y build-essential bison flex libgmp3-dev libmpc-dev libmpfr-dev texinfo wget


###############################################
###############################################
FROM development-base AS ci

###############################################
FROM development-base AS devcontainer
ARG BINUTILS_VERSION

COPY --from=build-tools-utils /tmp/output /usr/local/bin/
COPY --from=builder-binutils /tmp/binutils-build/ /tmp/binutils-build
COPY --from=builder-binutils /tmp/binutils-${BINUTILS_VERSION} /tmp/binutils-${BINUTILS_VERSION}

RUN <<EOF
	(
		cd /tmp/binutils-build
		make install
	)
	rm -rf /tmp/binutils-build "/tmp/binutils-${BINUTILS_VERSION}"
EOF
