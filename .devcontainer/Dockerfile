# syntax=docker/dockerfile:1

# renovate: datasource=github-releases packageName=junegunn/fzf
ARG FZF_VERSION='0.61.1'
# renovate: datasource=github-releases packageName=hadolint/hadolint
ARG HADOLINT_VERSION='2.12.0'

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
	rm -rf /var/lib/apt/lists/*
EOF

###############################################
###############################################
FROM development-base AS ci

###############################################
FROM development-base AS devcontainer

COPY --from=build-tools-utils /tmp/output /usr/local/bin/
