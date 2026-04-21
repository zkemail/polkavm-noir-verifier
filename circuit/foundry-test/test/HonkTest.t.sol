// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;
import "forge-std/Test.sol";
import "../src/HonkVerifier.sol";

contract HonkTest is Test {
    function test_verify() public {
        HonkVerifier v = new HonkVerifier();
        bytes memory proof = vm.readFileBinary("../target/proof");
        bytes32[] memory pub = new bytes32[](1);
        bytes memory pi = vm.readFileBinary("../target/public_inputs");
        assembly { mstore(add(pub, 32), mload(add(pi, 32))) }
        bool ok = v.verify(proof, pub);
        assertTrue(ok, "Solidity verifier failed");
    }
}
