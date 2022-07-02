// SPDX-License-Identifier: MIT
pragma solidity >=0.8.0 <0.9.0;

import "@openzeppelin/contracts/access/Ownable.sol";
import "./interfaces/IPriceFeed.sol";
import "./interfaces/IMint.sol";
import "./sAsset.sol";
import "./EUSD.sol";

contract Mint is Ownable, IMint{

    struct Asset {
        address token;
        uint minCollateralRatio;
        address priceFeed;
    }

    struct Position {
        uint idx;
        address owner;
        uint collateralAmount;
        address assetToken;
        uint assetAmount;
    }

    mapping(address => Asset) _assetMap;
    uint _currentPositionIndex;
    mapping(uint => Position) _idxPositionMap;
    address public collateralToken;
    

    constructor(address collateral) {
        collateralToken = collateral;
    }

    function registerAsset(address assetToken, uint minCollateralRatio, address priceFeed) external override onlyOwner {
        require(assetToken != address(0), "Invalid assetToken address");
        require(minCollateralRatio >= 1, "minCollateralRatio must be greater than 100%");
        require(_assetMap[assetToken].token == address(0), "Asset was already registered");
        
        _assetMap[assetToken] = Asset(assetToken, minCollateralRatio, priceFeed);
    }

    function getPosition(uint positionIndex) external view returns (address, uint, address, uint) {
        require(positionIndex < _currentPositionIndex, "Invalid index");
        Position storage position = _idxPositionMap[positionIndex];
        return (position.owner, position.collateralAmount, position.assetToken, position.assetAmount);
    }

    function getMintAmount(uint collateralAmount, address assetToken, uint collateralRatio) public view returns (uint) {
        Asset storage asset = _assetMap[assetToken];
        (int relativeAssetPrice, ) = IPriceFeed(asset.priceFeed).getLatestPrice();
        uint8 decimal = sAsset(assetToken).decimals();
        uint mintAmount = collateralAmount * (10 ** uint256(decimal)) / uint(relativeAssetPrice) / collateralRatio ;
        return mintAmount;
    }

    function checkRegistered(address assetToken) public view returns (bool) {
        return _assetMap[assetToken].token == assetToken;
    }

    /* TODO: implement your functions here */
    function openPosition(uint collateralAmount, address assetToken, uint collateralRatio) external override{
        require(this.checkRegistered(assetToken), "register");
        require(collateralRatio >= _assetMap[assetToken].minCollateralRatio, "MCR Wrong");

        EUSD eusd = EUSD(collateralToken);
        eusd.transferFrom(msg.sender, address(this), collateralAmount);
        
        uint mintAmount = getMintAmount(collateralAmount, assetToken, collateralRatio);
        Position memory position = Position(_currentPositionIndex, msg.sender, collateralAmount, assetToken, mintAmount);
        _idxPositionMap[_currentPositionIndex] = position;
        _currentPositionIndex += 1;

        sAsset asset = sAsset(assetToken);
        asset.mint(msg.sender, mintAmount);


    }
    // memory: https://stackoverflow.com/questions/47253748/solidity-storage-struct-not-compiling
    // approve: 

    function closePosition(uint positionIndex) external override{
        Position storage position = _idxPositionMap[positionIndex];
        require(position.owner == msg.sender, "owns wrong");

        sAsset asset = sAsset(position.assetToken);
        asset.burn(msg.sender, position.assetAmount);

        EUSD eusd = EUSD(collateralToken);
        eusd.transfer(msg.sender, position.assetAmount);

        delete _idxPositionMap[positionIndex];
    }

    function deposit(uint positionIndex, uint collateralAmount) external override{
        Position memory position = _idxPositionMap[positionIndex];
        require(position.owner == msg.sender, "owns wrong");
        // position.collateralAmount += collateralAmount;
        _idxPositionMap[positionIndex].collateralAmount = position.collateralAmount + collateralAmount;

        EUSD eusd = EUSD(collateralToken);
        eusd.transferFrom(msg.sender, address(this), collateralAmount);
    
    }
    function withdraw(uint positionIndex, uint withdrawAmount) external override{

        Position memory position = _idxPositionMap[positionIndex];
        require(position.owner == msg.sender, "owns wrong");
        require(position.collateralAmount >= withdrawAmount, "too much");

        Asset storage asset = _assetMap[position.assetToken]; 
        (int relativeAssetPrice, ) = IPriceFeed(asset.priceFeed).getLatestPrice();
        uint8 decimal = sAsset(asset.token).decimals();
        uint assetValue = uint(relativeAssetPrice) * position.assetAmount;
        uint nweCollateralAmount = position.collateralAmount - withdrawAmount;
        uint newMCR = nweCollateralAmount * (10 ** uint256(decimal)) / assetValue ;
        require(newMCR >= _assetMap[position.assetToken].minCollateralRatio, "MCR error");

        EUSD eusd = EUSD(collateralToken);
        eusd.transfer(msg.sender, withdrawAmount);
        _idxPositionMap[positionIndex].collateralAmount = nweCollateralAmount;

    }
    

    function mint(uint positionIndex, uint mintAmount) external override{
        Position memory position = _idxPositionMap[positionIndex];
        require(position.owner == msg.sender, "owns wrong");

        Asset storage asset = _assetMap[position.assetToken]; 
        (int relativeAssetPrice, ) = IPriceFeed(asset.priceFeed).getLatestPrice();
        uint8 decimal = sAsset(asset.token).decimals();
        uint assetAmount = position.assetAmount + mintAmount;
        uint assetValue = uint(relativeAssetPrice) * assetAmount;
        uint newMCR = position.collateralAmount * (10 ** uint256(decimal)) / assetValue ;
        require(newMCR >= 2, "MCR error");

        sAsset sasset = sAsset(position.assetToken);
        sasset.mint(msg.sender, mintAmount);
        _idxPositionMap[positionIndex].assetAmount = position.assetAmount + mintAmount;

    }
    function burn(uint positionIndex, uint burnAmount) external override{
        Position memory position = _idxPositionMap[positionIndex];
        require(position.owner == msg.sender, "owns wrong");
        require(position.assetAmount >= burnAmount, "burn wrong");

        sAsset asset = sAsset(position.assetToken);
        asset.burn(msg.sender, burnAmount);
        _idxPositionMap[positionIndex].assetAmount = position.assetAmount - burnAmount;
    }

    
    

    


}