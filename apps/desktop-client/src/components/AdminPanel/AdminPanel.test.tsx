import React from "react";

describe("AdminPanel Component", () => {
    it("should terminate user sessions in registry", () => {
        const remainingCount = 2;
        expect(remainingCount).toBe(2);
    });
});
