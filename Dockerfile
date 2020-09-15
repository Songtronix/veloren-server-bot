FROM archlinux:20200908

RUN pacman -Syu --noconfirm
RUN pacman -S base-devel git git-lfs rustup --noconfirm
RUN git lfs install
RUN rustup set profile minimal
RUN rustup default stable

COPY . .

RUN cargo build --release

FROM archlinux:20200908

RUN pacman -Syu --noconfirm
RUN pacman -S base-devel git git-lfs rustup --noconfirm
RUN git lfs install
RUN rustup set profile minimal

COPY --from=0 target/release/veloren_server_bot .

CMD [ "sh", "-c", "RUST_BACKTRACE=1 ./veloren_server_bot" ]
