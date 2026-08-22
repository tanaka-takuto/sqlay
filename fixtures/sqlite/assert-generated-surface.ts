import {
  type sqliteBulkInsertOrderItems_Input,
  sqliteBulkInsertOrderItems,
  type sqliteDeleteOrderItem_Input,
  sqliteDeleteOrderItem,
  type sqliteFindOrderByMainId_Input,
  type sqliteFindOrderByMainId_Output,
  sqliteFindOrderByMainId,
  type sqliteFindOrdersByIds_Input,
  type sqliteFindOrdersByIds_Output,
  sqliteFindOrdersByIds,
  type sqliteInsertOrder_Input,
  sqliteInsertOrder,
  type sqliteListOrders_Input,
  type sqliteListOrders_Output,
  sqliteListOrders,
  type sqliteListOrderIdsAcrossStatuses_Input,
  type sqliteListOrderIdsAcrossStatuses_Output,
  sqliteListOrderIdsAcrossStatuses,
  type sqliteReplaceOrder_Input,
  sqliteReplaceOrder,
  type sqliteUpdateOrder_Input,
  sqliteUpdateOrder
} from "./generated/valid/sqlite_builders";

type Assert<T extends true> = T;
type IsExact<A, B> =
  (<T>() => T extends A ? 1 : 2) extends <T>() => T extends B ? 1 : 2
    ? (<T>() => T extends B ? 1 : 2) extends <T>() => T extends A ? 1 : 2
      ? true
      : false
    : false;

type ListOrdersInputContract = Assert<
  IsExact<
    sqliteListOrders_Input,
    {
      minId: string;
      statusFilter?: { $fragment: "sqliteByStatus"; status: string };
    }
  >
>;
type ListOrdersOutputContract = Assert<
  IsExact<
    sqliteListOrders_Output,
    {
      id: string;
      status: string;
      note: string | null;
      total: number;
    }[]
  >
>;
type ListOrdersReturnContract = Assert<
  IsExact<
    ReturnType<typeof sqliteListOrders>,
    { sql: string; params: readonly unknown[] }
  >
>;

type FindOrdersByIdsInputContract = Assert<
  IsExact<
    sqliteFindOrdersByIds_Input,
    { ids: readonly [{ id: string }, ...{ id: string }[]] }
  >
>;
type FindOrdersByIdsOutputContract = Assert<
  IsExact<
    sqliteFindOrdersByIds_Output,
    { id: string; customerEmail: string; active: boolean }[]
  >
>;
type FindOrdersByIdsReturnContract = Assert<
  IsExact<
    ReturnType<typeof sqliteFindOrdersByIds>,
    { sql: string; params: readonly unknown[] }
  >
>;

type FindOrderByMainIdInputContract = Assert<
  IsExact<sqliteFindOrderByMainId_Input, { id: number }>
>;
type FindOrderByMainIdOutputContract = Assert<
  IsExact<
    sqliteFindOrderByMainId_Output,
    { id: number; status: string }[]
  >
>;
type FindOrderByMainIdReturnContract = Assert<
  IsExact<
    ReturnType<typeof sqliteFindOrderByMainId>,
    { sql: string; params: readonly [number] }
  >
>;

type ListOrderIdsAcrossStatusesInputContract = Assert<
  IsExact<
    sqliteListOrderIdsAcrossStatuses_Input,
    { primaryStatus: string; fallbackStatus: string }
  >
>;
type ListOrderIdsAcrossStatusesOutputContract = Assert<
  IsExact<sqliteListOrderIdsAcrossStatuses_Output, { id: unknown | null }[]>
>;
type ListOrderIdsAcrossStatusesReturnContract = Assert<
  IsExact<
    ReturnType<typeof sqliteListOrderIdsAcrossStatuses>,
    { sql: string; params: readonly [string, string] }
  >
>;

type InsertOrderInputContract = Assert<
  IsExact<
    sqliteInsertOrder_Input,
    {
      id: string;
      customerEmail: string;
      status: string;
      note: string | null;
      total: number;
      active: boolean;
    }
  >
>;
type UpdateOrderInputContract = Assert<
  IsExact<sqliteUpdateOrder_Input, { status: string; id: string }>
>;
type DeleteOrderItemInputContract = Assert<
  IsExact<
    sqliteDeleteOrderItem_Input,
    {
      id: string;
      orderFilter?: { $fragment: "sqliteByOrderId"; orderId: string };
    }
  >
>;
type ReplaceOrderInputContract = Assert<
  IsExact<sqliteReplaceOrder_Input, sqliteInsertOrder_Input>
>;
type BulkInsertOrderItemsInputContract = Assert<
  IsExact<
    sqliteBulkInsertOrderItems_Input,
    {
      items: readonly [
        { id: string; orderId: string; sku: string; quantity: string },
        ...{ id: string; orderId: string; sku: string; quantity: string }[]
      ];
    }
  >
>;

type InsertOrderReturnContract = Assert<
  IsExact<
    ReturnType<typeof sqliteInsertOrder>,
    {
      sql: string;
      params: readonly [string, string, string, string | null, number, boolean];
    }
  >
>;
type UpdateOrderReturnContract = Assert<
  IsExact<
    ReturnType<typeof sqliteUpdateOrder>,
    { sql: string; params: readonly [string, string] }
  >
>;
type DeleteOrderItemReturnContract = Assert<
  IsExact<
    ReturnType<typeof sqliteDeleteOrderItem>,
    { sql: string; params: readonly unknown[] }
  >
>;
type ReplaceOrderReturnContract = Assert<
  IsExact<
    ReturnType<typeof sqliteReplaceOrder>,
    ReturnType<typeof sqliteInsertOrder>
  >
>;
type BulkInsertOrderItemsReturnContract = Assert<
  IsExact<
    ReturnType<typeof sqliteBulkInsertOrderItems>,
    { sql: string; params: readonly unknown[] }
  >
>;

void sqliteFindOrdersByIds;

export type SQLiteFixtureSurfaceContracts = [
  ListOrdersInputContract,
  ListOrdersOutputContract,
  ListOrdersReturnContract,
  FindOrdersByIdsInputContract,
  FindOrdersByIdsOutputContract,
  FindOrdersByIdsReturnContract,
  FindOrderByMainIdInputContract,
  FindOrderByMainIdOutputContract,
  FindOrderByMainIdReturnContract,
  ListOrderIdsAcrossStatusesInputContract,
  ListOrderIdsAcrossStatusesOutputContract,
  ListOrderIdsAcrossStatusesReturnContract,
  InsertOrderInputContract,
  UpdateOrderInputContract,
  DeleteOrderItemInputContract,
  ReplaceOrderInputContract,
  BulkInsertOrderItemsInputContract,
  InsertOrderReturnContract,
  UpdateOrderReturnContract,
  DeleteOrderItemReturnContract,
  ReplaceOrderReturnContract,
  BulkInsertOrderItemsReturnContract
];
