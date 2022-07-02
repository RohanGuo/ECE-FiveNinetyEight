// SPDX-License-Identifier: MIT
pragma solidity >=0.8.0 <0.9.0;

import "@chainlink/contracts/src/v0.8/interfaces/AggregatorV3Interface.sol";
import "./interfaces/IPriceFeed.sol";

contract BNB is IPriceFeed {
    AggregatorV3Interface internal priceFeed;

    constructor() {
        priceFeed = AggregatorV3Interface(0x8993ED705cdf5e84D0a3B754b5Ee0e1783fcdF16);
    }

    function getLatestPrice() external override view returns (int price, uint lastUpdatedTime) {
        (
            /*uint80 roundID*/,
            price,
            /*uint startedAt */,
            lastUpdatedTime, 
            /*uint8 answeredInRound */
        ) = priceFeed.latestRoundData();
        return (price, lastUpdatedTime);
    }
}

contract TSLA is IPriceFeed {
    AggregatorV3Interface internal priceFeed;

    constructor() {
        priceFeed = AggregatorV3Interface(0xb31357d152638fd1ae0853d24b9Ea81dF29E3EF2);
    }

    function getLatestPrice() external override view returns (int price, uint lastUpdatedTime) {
        (
            /*uint80 roundID*/,
            price,
            /*uint startedAt */,
            lastUpdatedTime, 
            /*uint8 answeredInRound */
        ) = priceFeed.latestRoundData();
        return (price, lastUpdatedTime);
    }
}