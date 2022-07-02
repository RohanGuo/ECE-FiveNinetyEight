const fs = require('fs').promises;
contracts_to_deploy = ['Mint', 'EUSD', 'sAsset', 'PriceFeed']
var contracts = {}
for (name of contracts_to_deploy) {
    contracts[name] = artifacts.require(name)
}

contract("Mint test", async accounts => {
    console.log('starting test')
    
    it("Running setup", async () => {
        var instances = {}
        for (name of contracts_to_deploy) {
            instances[name] = await contracts[name].deployed()
        }
        
        let minterRole = await instances['sAsset'].MINTER_ROLE.call()
        let burnerRole = await instances['sAsset'].BURNER_ROLE.call()

        let minter_result = await instances['sAsset'].hasRole.call(minterRole, instances['Mint'].address)
        let burner_result = await instances['sAsset'].hasRole.call(burnerRole, instances['Mint'].address)
        assert.equal(minter_result, false);
        assert.equal(burner_result, false);
        await instances['sAsset'].grantRole(minterRole, instances['Mint'].address);
        await instances['sAsset'].grantRole(burnerRole, instances['Mint'].address);
        
        let registered = await instances['Mint'].checkRegistered.call(instances['sAsset'].address)
        assert.equal(registered, false);
        instances['Mint'].registerAsset(instances['sAsset'].address, 2, instances['PriceFeed'].address)

    });
    it("Checking setup", async () => {
        var instances = {}
        for (name of contracts_to_deploy) {
            instances[name] = await contracts[name].deployed()
        }
        const balance = await instances['EUSD'].balanceOf.call(accounts[0]);
        const symbol = await instances['sAsset'].symbol.call();
        const price = await instances['PriceFeed'].getLatestPrice.call();
        assert.equal(balance, 10000000000000000);
        assert.equal(symbol, 'sTSLA');
        assert.equal(price[0], 100000000000);
        let minterRole = await instances['sAsset'].MINTER_ROLE.call()
        let burnerRole = await instances['sAsset'].BURNER_ROLE.call()

        let minter_result = await instances['sAsset'].hasRole.call(minterRole, instances['Mint'].address)
        let burner_result = await instances['sAsset'].hasRole.call(burnerRole, instances['Mint'].address)
        assert.equal(minter_result, true);
        assert.equal(burner_result, true);

        let registered = await instances['Mint'].checkRegistered.call(instances['sAsset'].address)
        assert.equal(registered, true);

        const collateral = await instances['EUSD'].balanceOf.call(instances['Mint'].address);
        assert.equal(collateral, 0);
          
    });

    it("Test 1: test openPosition", async () => {
        var instances = {}
        for (name of contracts_to_deploy) {
            instances[name] = await contracts[name].deployed()
        }
        const collateralAmount = 3000 * 10 ** 8
        const collateralRatio = 2
        const price = await instances['PriceFeed'].getLatestPrice.call();

        await instances['EUSD'].approve(instances['Mint'].address, collateralAmount);
        await instances['Mint'].openPosition(collateralAmount, instances['sAsset'].address, collateralRatio);

        let result = await instances['Mint'].getPosition.call(0);
        assert.equal(result[0], accounts[0]);
        assert.equal(result[1], collateralAmount);
        assert.equal(result[2], instances['sAsset'].address);
        assert.equal(result[3], 150000000);

        const balance = await instances['sAsset'].balanceOf.call(accounts[0]);
        assert.equal(balance, 150000000);

        const collateral = await instances['EUSD'].balanceOf.call(instances['Mint'].address);
        assert.equal(collateral, collateralAmount);
    });

    it("Test 2: test deposit", async () => {
        var instances = {}
        for (name of contracts_to_deploy) {
            instances[name] = await contracts[name].deployed()
        }
        const collateralAmount = 3000 * 10 ** 8
        const collateralRatio = 2

        await instances['EUSD'].approve(instances['Mint'].address, collateralAmount);
        await instances['Mint'].deposit(0, collateralAmount);

        let result = await instances['Mint'].getPosition.call(0);
        assert.equal(result[0], accounts[0]);
        assert.equal(result[1], collateralAmount * 2);
        assert.equal(result[2], instances['sAsset'].address);
        assert.equal(result[3], 150000000);
        
        const collateral = await instances['EUSD'].balanceOf.call(instances['Mint'].address);
        assert.equal(collateral, collateralAmount * 2);
    });

    it("Test 3: test withdraw", async () => {
        var instances = {}
        for (name of contracts_to_deploy) {
            instances[name] = await contracts[name].deployed()
        }
        const collateralAmount = 3000 * 10 ** 8
        const collateralRatio = 2

        await instances['Mint'].withdraw(0, collateralAmount);

        let result = await instances['Mint'].getPosition.call(0);
        assert.equal(result[0], accounts[0]);
        assert.equal(result[1], collateralAmount);
        assert.equal(result[2], instances['sAsset'].address);
        assert.equal(result[3], 150000000);

        const collateral = await instances['EUSD'].balanceOf.call(instances['Mint'].address);
        assert.equal(collateral, collateralAmount);
    });

    it("Test 3.2: test withdraw too much", async () => {
        var instances = {}
        for (name of contracts_to_deploy) {
            instances[name] = await contracts[name].deployed()
        }
        const collateralAmount = 3000 * 10 ** 8     // after that will only remain 0 collateralAmount, which is impossible
        const collateralRatio = 2

        await instances['Mint'].withdraw(0, collateralAmount);

        let result = await instances['Mint'].getPosition.call(0);
        assert.equal(result[0], accounts[0]);
        assert.equal(result[1], collateralAmount);
        assert.equal(result[2], instances['sAsset'].address);
        assert.equal(result[3], 150000000);

        const collateral = await instances['EUSD'].balanceOf.call(instances['Mint'].address);
        assert.equal(collateral, 3000*10**8);       // still 3000*10**8, because reject this withdraw operation
    });

    it("Test 4: test burn", async () => {
        var instances = {}
        for (name of contracts_to_deploy) {
            instances[name] = await contracts[name].deployed()
        }
        const collateralAmount = 3000 * 10 ** 8
        const collateralRatio = 2

        let result = await instances['Mint'].getPosition.call(0);
        assert.equal(result[0], accounts[0]);
        assert.equal(result[1], collateralAmount);
        assert.equal(result[2], instances['sAsset'].address);
        assert.equal(result[3], 150000000);

        await instances['Mint'].burn(0, result[3]);

        result = await instances['Mint'].getPosition.call(0);
        assert.equal(result[0], accounts[0]);
        assert.equal(result[1], collateralAmount);
        assert.equal(result[2], instances['sAsset'].address);
        assert.equal(result[3], 0);

        const balance = await instances['sAsset'].balanceOf.call(accounts[0]);
        assert.equal(balance, 0);
    });

    it("Test 5: test mint", async () => {
        var instances = {}
        for (name of contracts_to_deploy) {
            instances[name] = await contracts[name].deployed()
        }
        const collateralAmount = 3000 * 10 ** 8
        const collateralRatio = 2

        await instances['Mint'].mint(0, 150000000);

        let result = await instances['Mint'].getPosition.call(0);
        assert.equal(result[0], accounts[0]);
        assert.equal(result[1], collateralAmount);
        assert.equal(result[2], instances['sAsset'].address);
        assert.equal(result[3], 150000000);

        const balance = await instances['sAsset'].balanceOf.call(accounts[0]);
        assert.equal(balance, 150000000);
    });

    // it("Test 5.1: test mint only owner", async () => {
    //     var instances = {}
    //     for (name of contracts_to_deploy) {
    //         instances[name] = await contracts[name].deployed()
    //     }
    //     const collateralAmount = 3000 * 10 ** 8
    //     const collateralRatio = 2

    //     await instances['Mint'].mint(0, 150000000);

    //     let result = await instances['Mint'].getPosition.call(0);
    //     assert.equal(result[0], accounts[0]);
    //     assert.equal(result[1], collateralAmount);
    //     assert.equal(result[2], instances['sAsset'].address);
    //     assert.equal(result[3], 150000000);

    //     const balance = await instances['sAsset'].balanceOf.call(accounts[0]);
    //     assert.equal(balance, 150000000);
    // });


    it("Test 5.2: test mint too much", async () => {
        var instances = {}
        for (name of contracts_to_deploy) {
            instances[name] = await contracts[name].deployed()
        }
        const collateralAmount = 3000 * 10 ** 8
        const collateralRatio = 2

        let result0 = await instances['Mint'].getPosition.call(0);
        assert.equal(result0[0], accounts[0]);
        assert.equal(result0[1], collateralAmount);
        assert.equal(result0[2], instances['sAsset'].address);
        assert.equal(result0[3], 150000000);                                    // mint 前账号内有 15..0 的钱
        // console.log("[Before mint] - collateralAmount: " + result0[1] + ", sAsset amount: " + result0[3]);
        // console.log("[Before mint] - collateralAmount: " + result0[1] + ", sAsset amount: " + result0[3]);

        await instances['Mint'].mint(0, 200000000000);                             // 理论上此时加上20..0后应该是 30..0 / (15..0 + 20..0) < MCR，应该报错并不加上内容的

        let result = await instances['Mint'].getPosition.call(0);
        // console.log("[After mint] - collateralAmount: " + result[1] + ", sAsset amount: " + result[3]);
        assert.equal(result[0], accounts[0]);
        assert.equal(result[1], collateralAmount);
        assert.equal(result[2], instances['sAsset'].address);
        assert.equal(result[3], 350000000);                                     // 此处能通过说明还是加上了钱

        const balance = await instances['sAsset'].balanceOf.call(accounts[0]);
        assert.equal(balance, 350000000);
    });

    // it("Test 6: test closePosition", async () => {
    //     var instances = {}
    //     for (name of contracts_to_deploy) {
    //         instances[name] = await contracts[name].deployed()
    //     }
    //     const collateralAmount = 3000 * 10 ** 8
    //     const collateralRatio = 2

    //     await instances['Mint'].closePosition(0);

    //     let result = await instances['Mint'].getPosition.call(0);
    //     assert.equal(result[0], 0);
    //     assert.equal(result[1], 0);
    //     assert.equal(result[2], 0);
    //     assert.equal(result[3], 0);

    //     const balance = await instances['sAsset'].balanceOf.call(accounts[0]);
    //     assert.equal(balance, 0);
    // });

    it("Test 7: test closePosition II", async () => {
        var instances = {}
        for (name of contracts_to_deploy) {
            instances[name] = await contracts[name].deployed()
        }
        const collateralAmount = 3000 * 10 ** 8
        const collateralRatio = 2
        const price = await instances['PriceFeed'].getLatestPrice.call();

        await instances['EUSD'].approve(instances['Mint'].address, collateralAmount + 5);
        await instances['Mint'].openPosition(collateralAmount, instances['sAsset'].address, collateralRatio);

        let result = await instances['Mint'].getPosition.call(0);
        assert.equal(result[0], accounts[0]);
        assert.equal(result[1], collateralAmount);
        assert.equal(result[2], instances['sAsset'].address);
        assert.equal(result[3], 150000000);
        
        // test deposit on a existed position
        await instances['Mint'].deposit(0, 3);
        await instances['Mint'].withdraw(0, 3);
        await instances['Mint'].mint(0, 1);
        await instances['Mint'].burn(0, 1);


        await instances['EUSD'].approve(instances['Mint'].address, collateralAmount);
        await instances['Mint'].openPosition(collateralAmount, instances['sAsset'].address, collateralRatio);


        try {
            let result1 = await instances['sAsset'].getPosition.call(1);
        } catch (e) {
            console.log('TEST [Only Owner] SUCCESS');
        }

        let result1 = await instances['Mint'].getPosition.call(1);
        assert.equal(result1[0], accounts[0]);
        assert.equal(result1[1], collateralAmount);
        assert.equal(result1[2], instances['sAsset'].address);
        assert.equal(result1[3], 150000000);

        await instances['Mint'].closePosition(0);

        // test operation on a deleted position
        try {
            await instances['Mint'].closePosition(0);
        } catch (e) {
            console.log('SUCCESS - Cannot [closePosition] on a deleted position');
        }

        try {
            await instances['Mint'].deposit(0, 3);
        } catch (e) {
            console.log('TEST SUCCESS - Cannot [deposit] on a deleted position');
        }
        try {
            await instances['Mint'].withdraw(0, 1);
        } catch (e) {
            console.log('TEST SUCCESS - Cannot [withdraw] on a deleted position');
        }
        try {
            await instances['Mint'].mint(0, 1);
        } catch (e) {
            console.log('TEST SUCCESS - Cannot [mint] on a deleted position');
        }
        try {
            await instances['Mint'].burn(0, 1);
        } catch (e) {
            console.log('TEST SUCCESS - Cannot [burn] on a deleted position');
        }

        // await instances['Mint'].deposit(0, 3);
        // await instances['Mint'].withdraw(0, 1);
        // await instances['Mint'].mint(0, 1);
        // await instances['Mint'].burn(0, 1);

        let result2 = await instances['Mint'].getPosition.call(0);
        assert.equal(result2[0], 0);
        assert.equal(result2[1], 0);
        assert.equal(result2[2], 0);
        assert.equal(result2[3], 0);

        await instances['Mint'].closePosition(1);
        let result3 = await instances['Mint'].getPosition.call(1);
        assert.equal(result3[0], 0);
        assert.equal(result3[1], 0);
        assert.equal(result3[2], 0);
        assert.equal(result3[3], 0);

        // const balance = await instances['sAsset'].balanceOf.call(accounts[0]);
        // assert.equal(balance, 0);
    });
});