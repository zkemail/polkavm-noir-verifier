// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

import "forge-std/Test.sol";
import "forge-std/console.sol";
import "../src/HonkVerifier.sol";

/// @dev Exposes accumulateRelationEvaluationsRaw via a public function so tests can retrieve
///      the 26 raw sub-relation evaluations for a given proof.
contract HonkVerifierDiag is HonkVerifier {
    function getRelationEvals(bytes calldata proof, bytes32[] calldata publicInputs)
        external
        view
        returns (
            Fr[NUMBER_OF_SUBRELATIONS] memory evals,
            Fr powPartialEval,
            Fr[5] memory gateChallenges,
            Fr[5] memory sumcheckUChallenges
        )
    {
        Honk.VerificationKey memory vk = loadVerificationKey();
        Honk.Proof memory p = TranscriptLib.loadProof(proof);

        Transcript memory t = TranscriptLib.generateTranscript(
            p, publicInputs, vk.circuitSize, vk.publicInputsSize, /*pubInputsOffset=*/1
        );

        t.relationParameters.publicInputsDelta = computePublicInputDelta(
            publicInputs, t.relationParameters.beta, t.relationParameters.gamma, /*pubInputsOffset=*/1
        );

        for (uint256 i = 0; i < 5; i++) {
            gateChallenges[i] = t.gateChallenges[i];
            sumcheckUChallenges[i] = t.sumCheckUChallenges[i];
        }

        // Replicate the sumcheck loop just to accumulate powPartialEvaluation
        Fr powPartialEvaluation = Fr.wrap(1);
        for (uint256 round = 0; round < logN; ++round) {
            Fr roundChallenge = t.sumCheckUChallenges[round];
            powPartialEvaluation = partiallyEvaluatePOW(
                t.gateChallenges[round], powPartialEvaluation, roundChallenge
            );
        }

        powPartialEval = powPartialEvaluation;
        evals = RelationsLib.accumulateRelationEvaluationsRaw(
            p.sumcheckEvaluations, t.relationParameters, powPartialEvaluation
        );
    }
}

contract RelationDiagTest is Test {
    string[26] NAMES = [
        "arith[0]",    // 0
        "arith[1]",    // 1
        "perm[0]",     // 2
        "perm[1]",     // 3
        "lookup[0]",   // 4
        "lookup[1]",   // 5
        "range[0]",    // 6
        "range[1]",    // 7
        "range[2]",    // 8
        "range[3]",    // 9
        "elliptic[0]", // 10
        "elliptic[1]", // 11
        "aux[0]",      // 12
        "aux[1]",      // 13
        "aux[2]",      // 14
        "aux[3]",      // 15
        "aux[4]",      // 16
        "aux[5]",      // 17
        "posext[0]",   // 18
        "posext[1]",   // 19
        "posext[2]",   // 20
        "posext[3]",   // 21
        "posint[0]",   // 22
        "posint[1]",   // 23
        "posint[2]",   // 24
        "posint[3]"    // 25
    ];

    function test_relation_evals() public {
        HonkVerifierDiag v = new HonkVerifierDiag();

        bytes memory proof = vm.readFileBinary("../target/proof");
        bytes32[] memory pub = new bytes32[](1);
        bytes memory pi = vm.readFileBinary("../target/public_inputs");
        assembly { mstore(add(pub, 32), mload(add(pi, 32))) }

        (Fr[NUMBER_OF_SUBRELATIONS] memory evals, Fr powPartialEval, Fr[5] memory gateChs, Fr[5] memory sumChs) = v.getRelationEvals(proof, pub);
        console.log("powPartialEvaluation:");
        console.logBytes32(bytes32(Fr.unwrap(powPartialEval)));
        console.log("gate_challenges[0..5]:");
        for (uint256 i = 0; i < 5; i++) {
            console.logBytes32(bytes32(Fr.unwrap(gateChs[i])));
        }
        console.log("sumcheck_u_challenges[0..5]:");
        for (uint256 i = 0; i < 5; i++) {
            console.logBytes32(bytes32(Fr.unwrap(sumChs[i])));
        }

        console.log("=== Reference sub-relation evaluations from Solidity ===");
        bool allZero = true;
        for (uint256 i = 0; i < NUMBER_OF_SUBRELATIONS; i++) {
            uint256 val = Fr.unwrap(evals[i]);
            console.log(string.concat("  evals[", vm.toString(i), "] ", NAMES[i], " = "), val);
            if (val != 0) allZero = false;
        }

        if (allZero) {
            console.log("ALL ZERO - grand sum will match round_target");
        } else {
            console.log("NON-ZERO evals found - these are the bugs to fix in Rust");
        }

        // The grand sum should equal zero for a valid proof's contribution
        // (roundTarget == grandSum is what verifySumcheck checks)
        // So all evals should be 0 before scaling only if roundTarget happens to == grandSum
        // More precisely: the BATCHED grand sum should equal roundTarget
        // Let's also print the grand sum (batched with alphas)
        // We don't have alphas here but the raw evals tell us which relations are wrong

        assertTrue(true); // Always pass - this is a diagnostic test
    }

    function test_verify_still_passes() public {
        HonkVerifier v = new HonkVerifier();
        bytes memory proof = vm.readFileBinary("../target/proof");
        bytes32[] memory pub = new bytes32[](1);
        bytes memory pi = vm.readFileBinary("../target/public_inputs");
        assembly { mstore(add(pub, 32), mload(add(pi, 32))) }
        bool ok = v.verify(proof, pub);
        assertTrue(ok, "Solidity verifier failed");
    }
}
