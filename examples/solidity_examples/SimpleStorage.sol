// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

contract SimpleStorage {
    uint256 private value;
    event ValueSet(uint256 newValue);

    constructor(uint256 initialValue) {
        value = initialValue;
    }

    function setValue(uint256 newValue) public {
        value = newValue;
        emit ValueSet(newValue);
    }

    function getValue() public view returns (uint256) {
        return value;
    }
} 