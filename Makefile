
main:
	cargo run -- --indir="packages"

prod:
	cargo run --release -- --indir="packages"

build:
	cargo build --release

run:
	./target/release/ecas-encoder --indir="packages"

clean:
	rm -rf out .cache
