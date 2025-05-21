---@class MxaSubprocess
---@field spawn fun():nil
---@field write_in fun(data: string):nil
---@field read_out fun():string
---@field read_err fun():string
---@field close_stdin fun():nil
---@field read_out_to_end fun():string
---@field read_err_to_end fun():string
---@field wait fun():number
---@field terminate fun():nil
---@field kill fun():nil

---@class MxaFetchRequest
---@field url string
---@field method string
---@field headers table
---@field body string
---@field output string

---@class MxaFetchResponse
---@field ok boolean
---@field status number
---@field status_text string
---@field headers table
---@field text string
---@field json table
---@field length integer
---@field output string
---@field error string

---@class MxaJson
---@field encode fun(data: string):table
---@field decode fun(data: table):string

---@class MxaScriptModule
---@field create_subprocess fun(program:string, args:string[]):MxaSubprocess
---@field run_with_args fun(program:string, args:string[]):(string, string, number)
---@field fetch fun(req: MxaFetchRequest):MxaFetchResponse
---@field json MxaJson

---@type MxaScriptModule
mxa = mxa