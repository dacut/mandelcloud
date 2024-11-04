FROM amazonlinux:2023 AS builder
RUN dnf install -y binutils diffutils gcc m4 util-linux-user
RUN useradd --home /builder --create-home --system --shell /bin/nologin builder
USER builder
WORKDIR /builder
COPY etc/rustup.sh /builder/rustup.sh
RUN /bin/sh /builder/rustup.sh -y --default-toolchain nightly
ENV PATH="/builder/.cargo/bin:${PATH}"

COPY Cargo.toml Cargo.lock rustfmt.toml LICENSE /builder/
COPY crates /builder/crates
WORKDIR /builder/crates/mandelcloud-compute
RUN cargo build --release
RUN /bin/ldd /builder/target/release/mandelcloud-compute

FROM public.ecr.aws/lambda/provided:al2023
COPY --from=builder /builder/target/release/mandelcloud-compute /bootstrap
ENTRYPOINT ["/bootstrap"]
