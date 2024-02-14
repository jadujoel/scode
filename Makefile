
main:
	cargo run -- --indir="/Users/admin/workspace/sounds/src/packages" --outfile="out/foo/sounds.txt"

prod:
	cargo run --release -- --indir="/Users/admin/workspace/sounds/src/packages" --outfile="out/foo/sounds.txt"

build:
	cargo build --release

run:
	./target/release/ecas-encoder --indir="/Users/admin/workspace/sounds/src/packages" --outfile="out/foo/sounds.txt"

clean:
	rm -rf out
