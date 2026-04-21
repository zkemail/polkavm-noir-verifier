// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.0;

interface IHonkVerifier {
    /// Verify an UltraHonk proof against the baked-in verification key.
    /// @param proof The raw proof bytes (from `bb prove`)
    /// @param publicInputs The public inputs as bytes32 array
    /// @return 0x01 if valid, 0x00 if invalid
    function verify(bytes calldata proof, bytes32[] calldata publicInputs) external view returns (bytes1);
}
