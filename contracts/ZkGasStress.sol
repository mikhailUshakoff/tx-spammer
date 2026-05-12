// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

// Precompile inputs are aligned with
// new-zkevm-root/execution-specs/tests/benchmark/marginal/test_marginal_precompiles.py
// so that block-level zk-gas measurements taken with this contract can be compared
// directly against the marginal-cost methodology there.
contract ZkGasStress {
    function stressAdd(uint256 iters) external pure {
        assembly {
            let x := 1
            for { let i := 0 } lt(i, iters) { i := add(i, 1) } {
                x := add(x, 1)
            }
            if eq(x, 0) { revert(0, 0) }
        }
    }

    function stressMulmod(uint256 iters) external pure {
        assembly {
            let a := 0xdeadbeefcafebabe1234567890abcdef
            let b := 0xfeedfacef00dbabe0badc0deabad1dea
            let n := 0xfffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffd
            for { let i := 0 } lt(i, iters) { i := add(i, 1) } {
                a := mulmod(a, b, n)
            }
            if eq(a, 0) { revert(0, 0) }
        }
    }

    function stressKeccak(uint256 iters) external view {
        assembly {
            let p := mload(0x40)
            mstore(p, 0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef)
            for { let i := 0 } lt(i, iters) { i := add(i, 1) } {
                let h := keccak256(p, 0x20)
                mstore(p, h)
            }
        }
    }

    // ECRECOVER (0x01) — same test vector as test_marginal_precompiles.py ECRECOVER_INPUT.
    function stressEcrecover(uint256 iters) external view {
        assembly {
            let p := mload(0x40)
            mstore(p,           0x456e9aea5e197a1f1af7a3e85a3212fa4049a3ba34c2289b4c860fc0b0c64ef3)
            mstore(add(p,0x20), 28)
            mstore(add(p,0x40), 0x9242685bf161793cc25603c231bc2f568eb630ea16aa137d2664ac8038825608)
            mstore(add(p,0x60), 0x4f8ae3bd7535248d0bd448298cc2e2071e56992d0774dc340c368ae950852ada)
            for { let i := 0 } lt(i, iters) { i := add(i, 1) } {
                let ok := staticcall(gas(), 0x01, p, 0x80, 0, 0)
                if iszero(ok) { revert(0, 0) }
            }
        }
    }

    // BLAKE2F (0x09) — input matches test_marginal_precompiles.py BLAKE2F_INPUT:
    //   rounds = 0xFFFF (max, ~65,535 gas/call)
    //   h      = standard BLAKE2 IV
    //   m      = "abc" + zero padding (128 bytes)
    //   t      = 0x03 || 0x00*15
    //   f      = 1 (final block)
    function stressBlake2f(uint256 iters) external view {
        assembly {
            let p := mload(0x40)
            // rounds (4 bytes BE @ offset 0): 0x0000FFFF, shifted into the top 4 bytes of a word
            mstore(p, shl(224, 0xFFFF))
            // h (64 bytes @ offset 4): standard BLAKE2 IV
            mstore(add(p,  4), 0x48c9bdf267e6096a3ba7ca8485ae67bb2bf894fe72f36e3cf1361d5f3af54fa5)
            mstore(add(p, 36), 0xd182e6ad7f520e511f6c3e2b8c68059b6bbd41fbabd9831f79217e1319cde05b)
            // m (128 bytes @ offset 68): "abc" + 125 zero bytes
            mstore(add(p,  68), 0x6162630000000000000000000000000000000000000000000000000000000000)
            mstore(add(p, 100), 0)
            mstore(add(p, 132), 0)
            mstore(add(p, 164), 0)
            // t (16 bytes @ offset 196): 0x03 followed by 15 zero bytes
            // We mstore 32 bytes here; the trailing 16 bytes spill into the f slot and beyond,
            // but we explicitly set f next so the overlap is harmless.
            mstore(add(p, 196), 0x0300000000000000000000000000000000000000000000000000000000000000)
            // f (1 byte @ offset 212): 1 (final block flag)
            mstore8(add(p, 212), 1)

            for { let i := 0 } lt(i, iters) { i := add(i, 1) } {
                let ok := staticcall(gas(), 0x09, p, 213, 0, 0)
                if iszero(ok) { revert(0, 0) }
            }
        }
    }

    // MODEXP (0x05) — Nagydani-5-qube worst case from test_marginal_precompiles.py:
    //   base_length = 512 (4096-bit)
    //   exp_length  = 1
    //   mod_length  = 512 (4096-bit)
    //   exponent    = 0x03
    //   ~698,709 gas/call (EIP-2565), stresses large-operand memory handling.
    function stressModexp(uint256 iters) external view {
        assembly {
            let p := mload(0x40)
            // Length headers (96 bytes)
            mstore(p,           0x200)  // base_length = 512
            mstore(add(p, 0x20), 0x01)  // exp_length  = 1
            mstore(add(p, 0x40), 0x200) // mod_length  = 512

            // base (512 bytes @ offset 0x60..0x260) — Nagydani-5 base
            mstore(add(p, 0x060), 0xc5a1611f8be90071a43db23cc2fe01871cc4c0e8ab5743f6378e4fef77f7f6db)
            mstore(add(p, 0x080), 0x0095c0727e20225beb665645403453e325ad5f9aeb9ba99bf3c148f63f9c07cf)
            mstore(add(p, 0x0a0), 0x4fe8847ad5242d6b7d4499f93bd47056ddab8f7dee878fc2314f344dbee2a7c4)
            mstore(add(p, 0x0c0), 0x1a5d3db91eff372c730c2fdd3a141a4b61999e36d549b9870cf2f4e632c4d5df)
            mstore(add(p, 0x0e0), 0x5f024f81c028000073a0ed8847cfb0593d36a47142f578f05ccbe28c0c06aeb1)
            mstore(add(p, 0x100), 0xb1da027794c48db880278f79ba78ae64eedfea3c07d10e0562668d839749dc95)
            mstore(add(p, 0x120), 0xf40467d15cf65b9cfc52c7c4bcef1cda3596dd52631aac942f146c7cebd46065)
            mstore(add(p, 0x140), 0x131699ce8385b0db1874336747ee020a5698a3d1a1082665721e769567f57983)
            mstore(add(p, 0x160), 0x0f9d259cec1a836845109c21cf6b25da572512bf3c42fd4b96e43895589042ab)
            mstore(add(p, 0x180), 0x60dd41f497db96aec102087fe784165bb45f942859268fd2ff6c012d9d00c02b)
            mstore(add(p, 0x1a0), 0xa83eace047cc5f7b2c392c2955c58a49f0338d6fc58749c9db2155522ac17914)
            mstore(add(p, 0x1c0), 0xec216ad87f12e0ee95574613942fa615898c4d9e8a3be68cd6afa4e7a003dedb)
            mstore(add(p, 0x1e0), 0xdf8edfee31162b174f965b20ae752ad89c967b3068b6f722c16b354456ba8e28)
            mstore(add(p, 0x200), 0x0f987c08e0a52d40a2e8f3a59b94d590aeef01879eb7a90b3ee7d772c839c855)
            mstore(add(p, 0x220), 0x19cbeaddc0c193ec4874a463b53fcaea3271d80ebfb39b33489365fc039ae549)
            mstore(add(p, 0x240), 0xa17a9ff898eea2f4cb27b8dbee4c17b998438575b2b8d107e4a0d66ba7fca85b)

            // exp (1 byte @ offset 0x260) = 0x03
            mstore8(add(p, 0x260), 0x03)

            // mod (512 bytes @ offset 0x261..0x461) — Nagydani-5 modulus.
            // Writes at unaligned byte offsets; mstore is byte-addressable so this is fine.
            mstore(add(p, 0x261), 0xe30049201ec12937e7ce79d0f55d9c810e20acf52212aca1d3888949e0e4830a)
            mstore(add(p, 0x281), 0xad88d804161230eb89d4d329cc83570fe257217d2119134048dd2ed167646975)
            mstore(add(p, 0x2a1), 0xfc7d77136919a049ea74cf08ddd2b896890bb24a0ba18094a22baa351bf29ad9)
            mstore(add(p, 0x2c1), 0x6c66bbb1a598f2ca391749620e62d61c3561a7d3653ccc8892c7b99baaf76bf8)
            mstore(add(p, 0x2e1), 0x36e2991cb06d6bc0514568ff0d1ec8bb4b3d6984f5eaefb17d3ea2893722375d)
            mstore(add(p, 0x301), 0x3ddb8e389a8eef7d7d198f8e687d6a513983df906099f9a2d23f4f9dec6f8ef2)
            mstore(add(p, 0x321), 0xf11fc0a21fac45353b94e00486f5e17d386af42502d09db33cf0cf28310e049c)
            mstore(add(p, 0x341), 0x07e88682aeeb00cb833c5174266e62407a57583f1f88b304b7c6e0c84bbe1c0f)
            mstore(add(p, 0x361), 0xd423072d37a5bd0aacf764229e5c7cd02473460ba3645cd8e8ae144065bf02d0)
            mstore(add(p, 0x381), 0xdd238593d8e230354f67e0b2f23012c23274f80e3ee31e35e2606a4a3f31d94a)
            mstore(add(p, 0x3a1), 0xb755e6d163cff52cbb36b6d0cc67ffc512aeed1dce4d7a0d70ce82f2baba12e8)
            mstore(add(p, 0x3c1), 0xd514dc92a056f994adfb17b5b9712bd5186f27a2fda1f7039c5df2c8587fdc62)
            mstore(add(p, 0x3e1), 0xf5627580c13234b55be4df3056050e2d1ef3218f0dd66cb05265fe1acfb0989d)
            mstore(add(p, 0x401), 0x8213f2c19d1735a7cf3fa65d88dad5af52dc2bba22b7abf46c3bc77b5091baab)
            mstore(add(p, 0x421), 0x9e8f0ddc4d5e581037de91a9f8dcbc69309be29cc815cf19a20a7585b8b3073e)
            mstore(add(p, 0x441), 0xdf51fc9baeb3e509b97fa4ecfd621e0fd57bd61cac1b895c03248ff12bdbc575)

            for { let i := 0 } lt(i, iters) { i := add(i, 1) } {
                let ok := staticcall(gas(), 0x05, p, 0x461, 0, 0)
                if iszero(ok) { revert(0, 0) }
            }
        }
    }
}
