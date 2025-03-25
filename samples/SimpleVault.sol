// SPDX-License-Identifier: MIT
pragma solidity ^0.8.17;

import "./SimpleToken.sol";

/**
 * @title SimpleVault
 * @dev A simple vault contract that interacts with the SimpleToken contract
 */
contract SimpleVault {
    SimpleToken public token;
    mapping(address => uint256) public deposits;
    
    event Deposit(address indexed user, uint256 amount);
    event Withdraw(address indexed user, uint256 amount);
    
    constructor(address _tokenAddress) {
        token = SimpleToken(_tokenAddress);
    }
    
    function deposit(uint256 _amount) external {
        require(_amount > 0, "Amount must be greater than 0");
        require(token.transferFrom(msg.sender, address(this), _amount), "Transfer failed");
        
        deposits[msg.sender] += _amount;
        emit Deposit(msg.sender, _amount);
    }
    
    function withdraw(uint256 _amount) external {
        require(_amount > 0, "Amount must be greater than 0");
        require(deposits[msg.sender] >= _amount, "Insufficient deposit");
        
        deposits[msg.sender] -= _amount;
        require(token.transfer(msg.sender, _amount), "Transfer failed");
        
        emit Withdraw(msg.sender, _amount);
    }
    
    function getBalance(address _user) external view returns (uint256) {
        return deposits[_user];
    }
} 