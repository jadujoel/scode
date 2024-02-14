
main:
	cargo run -- --indir="../sounds/src/packages" --outfile="out/foo/sounds.txt"

prod:
	cargo run --release -- --indir="../sounds/src/packages" --outfile="out/foo/sounds.txt"

build:
	cargo build --release

run:
	./target/release/ecas-encoder --indir="../sounds/src/packages" --outfile="out/foo/sounds.txt"

clean:
	rm -rf out
